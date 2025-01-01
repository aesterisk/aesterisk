use std::{fs, io, sync::{Arc, Mutex}, thread, time::{Duration, SystemTime}};

use config::Config;
use futures_channel::mpsc::{self, unbounded};
use futures_util::{future, join, pin_mut, FutureExt, StreamExt, TryStreamExt};
use josekit::{jwe::{self, alg::rsaes::{RsaesJweDecrypter, RsaesJweEncrypter}, JweHeader}, jwk::alg::rsa::RsaKeyPair, jwt::{self, JwtPayload, JwtPayloadValidator}};
use lazy_static::lazy_static;
use packet::{daemon_server::{auth::DSAuthPacket, handshake_response::DSHandshakeResponsePacket}, server_daemon::{auth_response::SDAuthResponsePacket, handshake_request::SDHandshakeRequestPacket, listen::SDListenPacket}, Packet, ID};
use tokio_tungstenite::tungstenite::{self, Message};
use tracing::{debug, error, info, warn, Level};
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod config;

type Rx = mpsc::UnboundedReceiver<Message>;
type Tx = mpsc::UnboundedSender<Message>;
type Sender = Arc<Mutex<Option<Tx>>>;

lazy_static! {
    static ref CONFIG: Config = config::load_or_create("config.toml");
    static ref DECRYPTER: josekit::jwe::alg::rsaes::RsaesJweDecrypter = get_decrypter(&CONFIG).expect("Failed to make decrypter");
    static ref ENCRYPTER: josekit::jwe::alg::rsaes::RsaesJweEncrypter = get_encrypter(&CONFIG).expect("Failed to make encrypter");
}

fn get_decrypter(config: &Config) -> Result<RsaesJweDecrypter, String> {
    match fs::read_to_string(&config.daemon.private_key) {
        Ok(pem) => {
            let decrypter = jwe::RSA_OAEP.decrypter_from_pem(pem.into_bytes()).map_err(|_| "Failed to parse PEM")?;
            info!("Loaded private RSA key from disk");
            Ok(decrypter)
        },
        Err(_) => {
            let key = RsaKeyPair::generate(2048).map_err(|_| "Failed to generate keys")?;
            fs::write(&config.daemon.private_key, key.to_pem_private_key()).map_err(|e| format!("Failed to save key to disk: {}", e))?;
            fs::write(&config.daemon.public_key, key.to_pem_public_key()).map_err(|e| format!("Failed to save key to disk: {}", e))?;
            info!("Generated RSA keys and saved to disk");
            Ok(jwe::RSA_OAEP.decrypter_from_pem(key.to_pem_private_key()).map_err(|_| "Failed to parse PEM")?)
        }
    }
}

fn get_encrypter(config: &Config) -> Result<RsaesJweEncrypter, String> {
    match fs::read_to_string(&config.server.public_key) {
        Ok(pem) => {
            let encrypter = jwe::RSA_OAEP.encrypter_from_pem(pem.into_bytes()).map_err(|_| "Failed to parse PEM")?;
            info!("Loaded public RSA key from disk");
            Ok(encrypter)
        },
        Err(_) => Err("Public key not specified".to_string())
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

    let _force_create = &DECRYPTER.clone();
    let _force_create = &ENCRYPTER.clone();

    if CONFIG.daemon.id.is_empty() {
        warn!("No Daemon ID set, please continue setup process!");
        return;
    }

    if let Err(_) = Uuid::parse_str(&CONFIG.daemon.id) {
        error!("Daemon ID is incorrectly set! Please check your config file.");
        return;
    }

    let sender = Arc::new(Mutex::new(None));

    let server_connector_handle = tokio::spawn(start_server_connector(sender.clone()));

    join!(server_connector_handle).0.expect("failed to join handle");

    warn!("Shutting down...");
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
    let (stream, _) = tokio_tungstenite::connect_async(&CONFIG.server.url).await.map_err(|e| format!("Could not connect to server: {}", error_to_string(e)))?;

    info!("Connected to server");
    let (write, read) = stream.split();

    info!("Authenticating...");
    tokio::spawn(handle_connection(sender.clone()).then(|res| match res {
        Ok(()) => future::ready(()),
        Err(e) => {
            error!("Error authenticating: {}", e);
            future::ready(())
        }
    }));

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

        tokio::spawn(handle_packet(text, sender.clone()).then(|res| match res {
            Ok(()) => future::ready(()),
            Err(e) => {
                error!("Error handling packet: {}", e);
                future::ready(())
            }
        }));
    });

    let outgoing = rx.map(Ok).forward(write);

    pin_mut!(incoming, outgoing);
    future::select(incoming, outgoing).await;

    Ok(())
}

fn encrypt_packet(packet: Packet, encrypter: &RsaesJweEncrypter) -> Result<String, String> {
    let mut header = JweHeader::new();
    header.set_token_type("JWT");
    header.set_algorithm("RSA-OAEP");
    header.set_content_encryption("A256GCM");

    let mut payload = JwtPayload::new();
    payload.set_claim("p", Some(serde_json::to_value(packet).map_err(|_| "packet should be serializable")?)).map_err(|_| "should set claim correctly")?;
    payload.set_issuer("aesterisk/daemon");
    payload.set_issued_at(&SystemTime::now());
    payload.set_expires_at(&SystemTime::now().checked_add(Duration::from_secs(60)).ok_or("duration overflow")?);

    Ok(jwt::encode_with_encrypter(&payload, &header, encrypter).map_err(|_| "could not encrypt token")?)
}

fn decrypt_packet(msg: &str, decrypter: &RsaesJweDecrypter, sender: Sender) -> Result<Packet, String> {
    let (payload, _) = jwt::decode_with_decrypter(msg, decrypter).expect("should decrypt");

    let mut validator = JwtPayloadValidator::new();
    validator.set_issuer("aesterisk/server");
    validator.set_base_time(SystemTime::now());
    validator.set_min_issued_time(SystemTime::now() - Duration::from_secs(60));
    validator.set_max_issued_time(SystemTime::now());

    match validator.validate(&payload) {
        Ok(()) => (),
        Err(e) => {
            sender.lock().map_err(|_| "sender has been poisoned")?.as_ref().ok_or("Client not found in channel_map")?.close_channel();
            return Err(format!("Invalid token: {}", e));
        }
    }

    // TODO: maybe don't clone hehe
    let try_packet = Packet::from_value(payload.claim("p").expect("should have .p").clone());

    Ok(try_packet.ok_or(format!("Could not parse packet: \"{}\"", msg))?)
}

async fn handle_connection(sender: Sender) -> Result<(), String> {
    sender.lock().map_err(|_| "sender has been poisoned")?.as_ref().ok_or("sender is not available")?.unbounded_send(
        Message::Text(
            encrypt_packet(
                DSAuthPacket {
                    daemon_uuid: CONFIG.daemon.id.clone()
                }.to_packet()?,
                &ENCRYPTER
            )?
        )
    ).map_err(|_| "Could not send message")?;

    Ok(())
}

async fn handle_packet(msg: String, sender: Sender) -> Result<(), String> {
    let packet = decrypt_packet(&msg, &DECRYPTER, sender.clone())?;

    debug!("Received Packet {:?}", packet.id);

    match packet.id {
        ID::SDAuthResponse => {
            handle_auth_response(SDAuthResponsePacket::parse(packet).expect("SDAuthResponsePacket should be Some")).await
        }
        ID::SDHandshakeRequest => {
            handle_handshake_request(SDHandshakeRequestPacket::parse(packet).expect("SDHandshakeRequestPacket should be Some"), sender.clone()).await
        }
        ID::SDListen => {
            handle_listen(SDListenPacket::parse(packet).expect("SDListenPacket should be Some")).await
        }
        _ => {
            Err(format!("Should not receive [A*|D*|SA] packet: {:?}", packet.id))
        }
    }
}

async fn handle_auth_response(auth_response_packet: SDAuthResponsePacket) -> Result<(), String> {
    if !auth_response_packet.success {
        return Err("Unsuccessful auth response".to_string());
    }

    info!("Authenticated");

    Ok(())
}

async fn handle_handshake_request(handshake_request_packet: SDHandshakeRequestPacket, sender: Sender) -> Result<(), String> {
    sender.lock().map_err(|_| "sender has been poisoned")?.as_ref().ok_or("sender is not available")?.unbounded_send(
        Message::Text(
            encrypt_packet(
                DSHandshakeResponsePacket {
                    challenge: handshake_request_packet.challenge,
                }.to_packet()?,
                &ENCRYPTER
            )?
        )
    ).map_err(|_| "Could not send message")?;

    Ok(())
}

async fn handle_listen(listen_packet: SDListenPacket) -> Result<(), String> {
    info!("Listen:\n{:#?}", listen_packet);

    Ok(())
}
