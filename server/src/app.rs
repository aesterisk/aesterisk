use std::{collections::HashMap, net::SocketAddr, sync::{Arc, RwLock}, time::{Duration, SystemTime}};

use futures_channel::mpsc::{self, unbounded};
use futures_util::{future, pin_mut, stream::{SplitSink, SplitStream}, FutureExt, StreamExt, TryStreamExt};
use josekit::{jwe::JweHeader, jwk::Jwk, jwt::{self, JwtPayload, JwtPayloadValidator}};
use reqwest::StatusCode;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite::{self, Message}, WebSocketStream};

use packet::{app_server::{auth::ASAuthPacket, listen::ASListenPacket}, server_app::auth_response::SAAuthResponsePacket, ListenEvent, Packet, ID};
use tracing::{debug, error, info, warn};

use crate::config::Config;

struct AppClient {
    user_id: u32,
    public_key: Vec<u8>,
    listens: Vec<ListenEvent>,
}

struct AppSocket {
    tx: Tx,
    authed: Option<AppClient>,
}

type Tx = mpsc::UnboundedSender<Message>;
type Rx = mpsc::UnboundedReceiver<Message>;
type ChannelMap = Arc<RwLock<HashMap<SocketAddr, AppSocket>>>;

pub async fn start(config: &'static Config, private_key: &'static Jwk) {
    let try_socket = TcpListener::bind(&config.sockets.app).await;
    let listener = match try_socket {
        Ok(listener) => listener,
        Err(e) => {
            error!("Error binding to socket: {}", e);
            return;
        }
    };

    info!("Listening on: {}", &config.sockets.app);

    let channel_map = ChannelMap::new(RwLock::new(HashMap::new()));

    loop {
        let conn = listener.accept().await;

        match conn {
            Ok((stream, addr)) => {
                tokio::spawn(accept_connection(stream, addr, channel_map.clone(), config, private_key).then(|res| match res {
                    Ok(_) => future::ready(()),
                    Err(e) => {
                        error!("Error in connection: {}", e);
                        future::ready(())
                    },
                }));
            }
            Err(e) => {
                error!("Error in connection: {}", e);
            }
        }
    }
}

// TODO: move to shared utils module
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

#[tracing::instrument(name = "app", skip(raw_stream, channel_map, config, private_key), fields(%addr))]
async fn accept_connection(raw_stream: TcpStream, addr: SocketAddr, channel_map: ChannelMap, config: &'static Config, private_key: &'static Jwk) -> Result<(), String> {
    info!("Accepted TCP connection");

    let stream = tokio_tungstenite::accept_async(raw_stream).await.map_err(|e| format!("Could not accept connection: {}", error_to_string(e)))?;

    let (write, read) = stream.split();

    let (tx, rx) = unbounded();
    channel_map.write().map_err(|_| "channel_map has been poisoned")?.insert(addr, AppSocket {
        tx,
        authed: None,
    });

    handle_client(write, read, addr, rx, channel_map, config, private_key).await
}

async fn handle_client(write: SplitSink<WebSocketStream<TcpStream>, Message>, read: SplitStream<WebSocketStream<TcpStream>>, addr: SocketAddr, rx: Rx, channel_map: ChannelMap, config: &'static Config, private_key: &'static Jwk) -> Result<(), String> {
    info!("Established WebSocket connection");

    let incoming = read.try_filter(|msg| future::ready(msg.is_text())).for_each(|msg| async {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                error!("Error reading message: {}", error_to_string(e));
                return;
            }
        };

        let text = match msg.into_text() {
            Ok(text) => text,
            Err(e) => {
                error!("Error converting message to text: {}", e);
                return;
            }
        };

        tokio::spawn(handle_packet(text, addr, channel_map.clone(), config, private_key).then(|res| match res {
            Ok(_) => future::ready(()),
            Err(e) => {
                error!("Error handling packet: {}", e);
                future::ready(())
            },
        }));
    });

    let outgoing = rx.map(Ok).forward(write);

    pin_mut!(incoming, outgoing);
    future::select(incoming, outgoing).await;

    channel_map.write().map_err(|_| "channel_map has been poisoned")?.remove(&addr);
    info!("Disconnected");

    Ok(())
}

#[tracing::instrument(name = "app", skip(msg, channel_map, config, private_key), fields(%addr))]
async fn handle_packet(msg: String, addr: SocketAddr, channel_map: ChannelMap, config: &Config, private_key: &'static Jwk) -> Result<(), String> {
    let decrypter = josekit::jwe::RSA_OAEP.decrypter_from_jwk(private_key).expect("decrypter should create successfully");

    let (payload, _) = jwt::decode_with_decrypter(&msg, &decrypter).expect("should decrypt");

    let mut validator = JwtPayloadValidator::new();
    validator.set_issuer("aesterisk/app");
    validator.set_base_time(SystemTime::now());
    validator.set_min_issued_time(SystemTime::now() - Duration::from_secs(60));
    validator.set_max_issued_time(SystemTime::now());

    validator.validate(&payload).expect("invalid token");

    // TODO: maybe don't clone hehe
    let try_packet = Packet::from_value(payload.claim("p").expect("should have .p").clone());

    let packet = try_packet.ok_or(format!("Could not parse packet: \"{}\"", msg))?;

    match packet.id {
        ID::ASAuth => {
            handle_auth(ASAuthPacket::parse(packet).expect("ASAuthPacket should be Some"), addr, channel_map, config).await
        }
        ID::ASListen => {
            handle_listen(ASListenPacket::parse(packet).expect("ASListenPacket should be Some"), addr, channel_map).await
        }
        _ => {
            Err(format!("Should not receive [SD]* packet: {:?}", packet.id))
        }
    }
}

fn encrypt_packet(packet: Packet, key: &Vec<u8>) -> String {
    let mut header = JweHeader::new();
    header.set_token_type("JWT");
    header.set_algorithm("RSA-OAEP");
    header.set_content_encryption("A256GCM");

    let mut payload = JwtPayload::new();
    payload.set_claim("p", Some(serde_json::to_value(packet).expect("packet should be serializable"))).expect("should set claim correctly");
    payload.set_issuer("aesterisk/server");
    payload.set_issued_at(&SystemTime::now());
    payload.set_expires_at(&SystemTime::now().checked_add(Duration::from_secs(60)).expect("this should not overflow (I hope)"));

    let encrypter = josekit::jwe::RSA_OAEP.encrypter_from_pem(key).expect("key should be valid");
    jwt::encode_with_encrypter(&payload, &header, &encrypter).expect("could not encrypt token")
}

async fn handle_auth(auth_packet: ASAuthPacket, addr: SocketAddr, channel_map: ChannelMap, config: &Config) -> Result<(), String> {
    let res = reqwest::Client::new()
        .get(String::from(&config.server.app_url) + "/api/verify")
        .query(&[("id", auth_packet.user_id)])
        .query(&[("key", &auth_packet.public_key)])
        .send().await.map_err(|e| format!("Could not reach aesterisk/app successfully: {}", e.to_string()))?;

    let mut clients = channel_map.write().map_err(|_| "channel_map has been poisoned")?;
    let client = clients.get_mut(&addr).ok_or("Client not found in channel_map")?;

    match res.status() {
        StatusCode::OK => {
            info!("Authenticated");

            let public_key = auth_packet.public_key.into_bytes();

            client.tx.unbounded_send(
                Message::text(
                    encrypt_packet(
                        SAAuthResponsePacket {
                            success: true,
                        }.to_packet(),
                        &public_key,
                    )
                )
            ).map_err(|_| "Failed to send packet")?;

            client.authed = Some(AppClient {
                user_id: auth_packet.user_id,
                public_key,
                listens: Vec::new(),
            });
        }
        _ => {
            warn!("Failed authentication");
            client.tx.close_channel();
        }
    }

    Ok(())
}

async fn handle_listen(listen_packet: ASListenPacket, addr: SocketAddr, channel_map: ChannelMap) -> Result<(), String> {
    let mut clients = channel_map.write().map_err(|_| "channel_map has been poisoned")?;
    let client = clients.get_mut(&addr).ok_or("Client not found in channel_map")?;

    for event in listen_packet.events.iter() {
        match event {
            ListenEvent::NodesList(nodes) => {
                debug!("Listening for NodesList: {:?}", nodes);
            }
        }
    }

    client.authed.as_mut().ok_or("Client not authenticated")?.listens = listen_packet.events;

    Ok(())
}
