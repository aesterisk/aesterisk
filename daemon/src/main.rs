use std::{io, sync::{Arc, Mutex}, thread, time::Duration};

use config::Config;
use futures_channel::mpsc::{self, unbounded};
use futures_util::{future, join, pin_mut, StreamExt, TryStreamExt};
use josekit::jwk::alg::rsa::RsaKeyPair;
use lazy_static::lazy_static;
use packet::{daemon_server::auth::DSAuthPacket, server_daemon::{auth_response::SDAuthResponsePacket, listen::SDListenPacket}, Packet, ID};
use tokio_tungstenite::tungstenite::{self, Message};
use tracing::{debug, error, info, warn, Level};
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt};

mod config;

type Rx = mpsc::UnboundedReceiver<Message>;
type Tx = mpsc::UnboundedSender<Message>;
type Sender = Arc<Mutex<Option<Tx>>>;

lazy_static! {
    static ref CONFIG: Config = config::load_or_create("config.toml");
    static ref PRIVATE_KEY: josekit::jwk::Jwk = read_key_or_exit(&CONFIG);
}

fn read_key_or_exit(config: &Config) -> josekit::jwk::Jwk {
    match read_key(config) {
        Ok(key) => key,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn read_key(config: &Config) -> Result<josekit::jwk::Jwk, String> {
    match std::fs::read_to_string(&config.daemon.private_key) {
        Ok(pem) => {
            let key = RsaKeyPair::from_pem(pem).map_err(|_| "Failed to parse PEM")?;
            info!("Loaded RSA key from disk");
            Ok(key.to_jwk_private_key())
        },
        Err(_) => {
            let key = RsaKeyPair::generate(2048).map_err(|_| "Failed to generate keys")?;
            std::fs::write(&config.daemon.private_key, key.to_pem_private_key()).map_err(|e| format!("Failed to save key to disk: {}", e))?;
            std::fs::write(&config.daemon.public_key, key.to_pem_public_key()).map_err(|e| format!("Failed to save key to disk: {}", e))?;
            info!("Generated RSA keys and saved to disk");
            Ok(key.to_jwk_private_key())
        }
    }
}

#[tokio::main]
async fn main() {
    // TODO: use clap for overriding options from config file
    // TODO: optionally pass in config path as argument to override default file
    
    let logs_rotation = tracing_appender::rolling::Builder::new().filename_suffix("daemon.aesterisk.log").rotation(Rotation::DAILY).build(&CONFIG.logging.folder).expect("could not initialize file logger");
    let (logs_file, _logs_file_guard) = tracing_appender::non_blocking(logs_rotation);
    let logs_file_layer = tracing_subscriber::fmt::layer().with_writer(logs_file.with_max_level(Level::DEBUG)).with_ansi(false);

    let (logs_stderr, _logs_stderr_guard) = tracing_appender::non_blocking(io::stderr());
    let (logs_stdout, _logs_stdout_guard) = tracing_appender::non_blocking(io::stdout());
    let logs_stdout_layer = tracing_subscriber::fmt::layer().with_writer(logs_stderr.with_max_level(Level::WARN).or_else(logs_stdout.with_max_level(Level::DEBUG))).with_ansi(true);

    tracing_subscriber::registry().with(logs_file_layer).with(logs_stdout_layer).init();

    info!("Starting Aesterisk Daemon v{}", env!("CARGO_PKG_VERSION"));

    let _force_create = &PRIVATE_KEY.to_string();

    if CONFIG.daemon.id.is_empty() {
        warn!("No Daemon ID set, please continue setup process!");
        return;
    }

    let sender = Arc::new(Mutex::new(None));

    let server_connector_handle = tokio::spawn(start_server_connector(sender.clone()));

    join!(server_connector_handle).0.expect("failed to join handle");
}

async fn start_server_connector(sender: Sender) {
    let mut attempts = 0;

    loop {
        if attempts <= 5 || attempts % 1800 == 0 {
            info!("Connecting to server...");
        }

        let (tx, rx) = unbounded();
        sender.lock().expect("failed to gain lock").replace(tx);

        let (join,) = join!(tokio::spawn(connect_to_server(rx, sender.clone())));

        match join {
            Ok(Ok(())) => {
                attempts = 1;
            },
            Ok(Err(e)) => if attempts <= 5 || attempts % 1800 == 0 {
                error!("{}", e);
            },
            Err(_) => if attempts <= 5 || attempts % 1800 == 0 {
                error!("Couldn't join connection handle");
            },
        }

        attempts += 1;
        
        // TODO: Implement exponential backoff
        // TODO: maybe add a limit to the amount of attempts
        // TODO: don't hardcode logging attempts
        if attempts <= 5 || attempts % 1800 == 0 {
            warn!("Disconnected from server, retrying... (attempt {})", attempts);
        } else if attempts == 6 {
            warn!("Max logged attempts reached, further attempts will be logged every 30 minutes (retrying in the background otherwise)"); // cuz 1800 secs = 30 min
        }

        thread::sleep(Duration::from_secs(1));
    }
}

fn error_to_string(e: tungstenite::Error) -> String {
    match e {
        tungstenite::Error::Utf8 => format!("Error in UTF-8 encoding"),
        tungstenite::Error::Io(e) => format!("IO error ({})", e.kind()),
        tungstenite::Error::Tls(_) => format!("TLS error"),
        tungstenite::Error::Url(_) => format!("Invalid URL"),
        tungstenite::Error::Http(_) => format!("HTTP error"),
        tungstenite::Error::HttpFormat(_) => format!("HTTP format error"),
        tungstenite::Error::Capacity(_) => format!("Buffer capacity exhausted"),
        tungstenite::Error::Protocol(_) => format!("Protocol violation"),
        tungstenite::Error::AlreadyClosed => format!("Connection already closed"),
        tungstenite::Error::AttackAttempt => format!("Attack attempt detected"),
        tungstenite::Error::WriteBufferFull(_) => format!("Write buffer full"),
        tungstenite::Error::ConnectionClosed => format!("Connection closed"),
    }
}

async fn connect_to_server(rx: Rx, sender: Sender) -> Result<(), String> {
    let (stream, _) = tokio_tungstenite::connect_async("ws://127.0.0.1:31304").await.map_err(|e| format!("Could not connect to server: {}", error_to_string(e)))?;

    info!("Connected to server");
    let (write, read) = stream.split();

    info!("Authenticating...");
    tokio::spawn(handle_connection(sender.clone()));

    let incoming = read.try_filter(|msg| future::ready(msg.is_text())).for_each(|msg| async {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                error!("{}", error_to_string(e));
                return;
            }
        };

        let text = match msg.into_text() {
            Ok(text) => text,
            Err(e) => {
                error!("{}", error_to_string(e));
                return;
            }
        };

        debug!("Message: {}", text);
        tokio::spawn(handle_packet(text));
    });

    let outgoing = rx.map(Ok).forward(write);

    pin_mut!(incoming, outgoing);
    future::select(incoming, outgoing).await;

    Ok(())
}

async fn handle_connection(sender: Sender) {
    let auth_packet = DSAuthPacket {
        id: 1,
        token: String::from("hi"),
    };

    let auth_packet_data = match auth_packet.to_string() {
        Ok(data) => data,
        Err(_) => {
            error!("Failed to serialize DSAuthPacket");
            return;
        }
    };

    sender.lock().expect("lock should not be poisoned").as_ref().expect("sender should be available").unbounded_send(Message::Text(auth_packet_data)).expect("message should get sent");
}

async fn handle_packet(msg: String) {
    debug!("Received packet");

    let try_packet = Packet::from_str(&msg);

    if try_packet.is_none() {
        return;
    }

    let packet = try_packet.expect("packet should be some");

    debug!("Packet:\n{:#?}", packet);

    match packet.id {
        ID::SDAuthResponse => {
            handle_auth_response(SDAuthResponsePacket::parse(packet).expect("SDAuthResponsePacket should be Some")).await;
        }
        ID::SDListen => {
            handle_listen(SDListenPacket::parse(packet).expect("SDListenPacket should be Some")).await;
        }
        _ => {
            eprintln!("(E) Should not receive [A*|D*|SA] packet: {:?}", packet.id);
        }
    }
}

async fn handle_auth_response(auth_response_packet: SDAuthResponsePacket) {
    info!("Auth Response:\n{:#?}", auth_response_packet);
}

async fn handle_listen(listen_packet: SDListenPacket) {
    info!("Listen:\n{:#?}", listen_packet);
}
