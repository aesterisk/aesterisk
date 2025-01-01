use std::{collections::HashMap, net::SocketAddr, sync::{Arc, Mutex, RwLock}, time::{Duration, SystemTime}};

use futures_channel::mpsc::{self, unbounded};
use futures_util::{future, pin_mut, stream::{SplitSink, SplitStream}, FutureExt, StreamExt, TryStreamExt};
use josekit::{jwe::{alg::rsaes::{RsaesJweDecrypter, RsaesJweEncrypter}, JweHeader}, jwt::{self, JwtPayload, JwtPayloadValidator}};
use openssl::rand::rand_bytes;
use sqlx::{types::Uuid, PgPool};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite::{self, Message}, WebSocketStream};

use packet::{daemon_server::{auth::DSAuthPacket, event::DSEventPacket, handshake_response::DSHandshakeResponsePacket}, server_daemon::{auth_response::SDAuthResponsePacket, handshake_request::SDHandshakeRequestPacket}, Packet, ID};
use tracing::{debug, error, info, warn};

use crate::{CONFIG, DECRYPTER};

struct DaemonClient;

struct DaemonHandshake {
    daemon_uuid: Uuid,
    encrypter: RsaesJweEncrypter,
    challenge: String,
}

struct DaemonSocket {
    tx: Tx,
    handshake: Option<DaemonHandshake>,
    authed: Option<DaemonClient>,
}

type Tx = mpsc::UnboundedSender<Message>;
type Rx = mpsc::UnboundedReceiver<Message>;
type ChannelMap = Arc<RwLock<HashMap<SocketAddr, DaemonSocket>>>;
type KeyCache = Arc<Mutex<HashMap<Uuid, Arc<Vec<u8>>>>>;

pub async fn start(pool: PgPool) {
    let try_socket = TcpListener::bind(&CONFIG.sockets.daemon).await;
    let listener = match try_socket {
        Ok(listener) => listener,
        Err(e) => {
            error!("Error binding to socket: {}", e);
            return;
        }
    };

    info!("Listening on: {}", &CONFIG.sockets.daemon);

    let channel_map = ChannelMap::new(RwLock::new(HashMap::new()));
    let key_cache = KeyCache::new(Mutex::new(HashMap::new()));

    loop {
        let conn = listener.accept().await;

        match conn {
            Ok((stream, addr)) => {
                tokio::spawn(accept_connection(stream, addr, channel_map.clone(), key_cache.clone(), pool.clone()).then(|res| match res {
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

#[tracing::instrument(name = "daemon", skip(raw_stream, channel_map, key_cache, pool), fields(%addr))]
async fn accept_connection(raw_stream: TcpStream, addr: SocketAddr, channel_map: ChannelMap, key_cache: KeyCache, pool: PgPool) -> Result<(), String> {
    info!("Accepted TCP connection");

    let stream = tokio_tungstenite::accept_async(raw_stream).await.map_err(|e| format!("Could not accept connection: {}", error_to_string(e)))?;

    let (write, read) = stream.split();

    let (tx, rx) = unbounded();
    channel_map.write().map_err(|_| "channel_map has been poisoned")?.insert(addr, DaemonSocket {
        tx,
        authed: None,
        handshake: None,
    });

    handle_client(write, read, addr, rx, channel_map, key_cache, pool).await
}


async fn handle_client(write: SplitSink<WebSocketStream<TcpStream>, Message>, read: SplitStream<WebSocketStream<TcpStream>>, addr: SocketAddr, rx: Rx, channel_map: ChannelMap, key_cache: KeyCache, pool: PgPool) -> Result<(), String> {
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

        tokio::spawn(handle_packet(text, addr, channel_map.clone(), key_cache.clone(), pool.clone()).then(|res| match res {
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

#[tracing::instrument(name = "daemon", skip(msg, channel_map, key_cache, pool), fields(%addr))]
async fn handle_packet(msg: String, addr: SocketAddr, channel_map: ChannelMap, key_cache: KeyCache, pool: PgPool) -> Result<(), String> {
    let packet = decrypt_packet(&msg, &DECRYPTER, channel_map.clone(), &addr)?;

    match packet.id {
        ID::DSAuth => {
            handle_auth(DSAuthPacket::parse(packet).expect("DSAuthPacket should be Some"), addr, channel_map, key_cache, pool).await
        },
        ID::DSHandshakeResponse => {
            handle_handshake_response(DSHandshakeResponsePacket::parse(packet).expect("DSHandshakeResponsePacket should be Some"), addr, channel_map).await
        }
        ID::DSEvent => {
            handle_event(DSEventPacket::parse(packet).expect("DSEventPacket should be Some"), addr, channel_map).await
        },
        _ => {
            Err(format!("Should not receive [SA]* packet: {:?}", packet.id))
        },
    }
}

fn encrypt_packet(packet: Packet, encrypter: &RsaesJweEncrypter) -> Result<String, String> {
    let mut header = JweHeader::new();
    header.set_token_type("JWT");
    header.set_algorithm("RSA-OAEP");
    header.set_content_encryption("A256GCM");

    let mut payload = JwtPayload::new();
    payload.set_claim("p", Some(serde_json::to_value(packet).map_err(|_| "packet should be serializable")?)).map_err(|_| "should set claim correctly")?;
    payload.set_issuer("aesterisk/server");
    payload.set_issued_at(&SystemTime::now());
    payload.set_expires_at(&SystemTime::now().checked_add(Duration::from_secs(60)).ok_or("duration overflow")?);

    Ok(jwt::encode_with_encrypter(&payload, &header, encrypter).map_err(|_| "could not encrypt token")?)
}

fn decrypt_packet(msg: &str, decrypter: &RsaesJweDecrypter, channel_map: ChannelMap, addr: &SocketAddr) -> Result<Packet, String> {
    let (payload, _) = jwt::decode_with_decrypter(&msg, decrypter).expect("should decrypt");

    let mut validator = JwtPayloadValidator::new();
    validator.set_issuer("aesterisk/daemon");
    validator.set_base_time(SystemTime::now());
    validator.set_min_issued_time(SystemTime::now() - Duration::from_secs(60));
    validator.set_max_issued_time(SystemTime::now());

    match validator.validate(&payload) {
        Ok(()) => (),
        Err(e) => {
            channel_map.write().map_err(|_| "channel_map has been poisoned")?.get(addr).ok_or("Client not found in channel_map")?.tx.close_channel();
            return Err(format!("Invalid token: {}", e));
        }
    }

    // TODO: maybe don't clone hehe
    let try_packet = Packet::from_value(payload.claim("p").expect("should have .p").clone());

    Ok(try_packet.ok_or(format!("Could not parse packet: \"{}\"", msg))?)
}

struct PublicKeyQuery {
    node_public_key: String,
}

async fn query_user_public_key(daemon_uuid: &Uuid, key_cache: KeyCache, pool: PgPool) -> Result<Arc<Vec<u8>>, String> {
    {
        let cache = key_cache.lock().map_err(|_| "key_cache has been poisoned")?;
        if let Some(v) = cache.get(daemon_uuid) {
            return Ok(v.clone());
        }
    }

    let res = sqlx::query_as!(PublicKeyQuery, "SELECT node_public_key FROM aesterisk.nodes WHERE node_uuid = $1", daemon_uuid).fetch_one(&pool).await.map_err(|_| format!("Node with UUID {} does not exist", &daemon_uuid))?;

    let mut cache = key_cache.lock().map_err(|_| "key_cache has been poisoned")?;
    cache.insert(daemon_uuid.clone(), Arc::new(res.node_public_key.into_bytes()));
    Ok(cache.get(daemon_uuid).ok_or("key should be in cache")?.clone())
}

async fn handle_auth(auth_packet: DSAuthPacket, addr: SocketAddr, channel_map: ChannelMap, key_cache: KeyCache, pool: PgPool) -> Result<(), String> {
    let mut challenge_bytes = [0; 256];
    rand_bytes(&mut challenge_bytes).map_err(|_| "Could not generate challenge")?;
    let challenge = challenge_bytes.iter().map(|byte| format!("{:02X}", byte)).collect::<String>();

    let uuid = Uuid::parse_str(&auth_packet.daemon_uuid).map_err(|_| "Could not parse UUID")?;
    let key = query_user_public_key(&uuid, key_cache, pool).await?;

    let mut clients = channel_map.write().map_err(|_| "channel_map has been poisoned")?;
    let client = clients.get_mut(&addr).ok_or("Client not found in channel_map")?;

    client.handshake = Some(DaemonHandshake {
        daemon_uuid: uuid,
        encrypter: josekit::jwe::RSA_OAEP.encrypter_from_pem(key.as_ref()).map_err(|_| "key should be valid")?,
        challenge: challenge.clone(),
    });

    client.tx.unbounded_send(
        Message::text(
            encrypt_packet(
                SDHandshakeRequestPacket {
                    challenge
                }.to_packet(),
                &client.handshake.as_ref().ok_or("Client hasn't requested authentication")?.encrypter,
            )?
        )
    ).map_err(|_| "Failed to send packet")?;

    Ok(())
}

async fn handle_handshake_response(handshake_reponse_packet: DSHandshakeResponsePacket, addr: SocketAddr, channel_map: ChannelMap) -> Result<(), String> {
    let mut clients = channel_map.write().map_err(|_| "channel_map has been poisoned")?;
    let client = clients.get_mut(&addr).ok_or("Client not found in channel_map")?;

    if handshake_reponse_packet.challenge != client.handshake.as_ref().ok_or("Client hasn't requested authentication")?.challenge {
        warn!("Failed authentication");
        client.tx.close_channel();
        return Err("Challenge does not match".to_string());
    }

    info!("Authenticated");

    client.tx.unbounded_send(
        Message::text(
            encrypt_packet(
                SDAuthResponsePacket {
                    success: true,
                }.to_packet()?,
                &client.handshake.as_ref().ok_or("Client hasn't requested authentication")?.encrypter,
            )?
        )
    ).map_err(|_| "Failed to send packet")?;

    client.authed = Some(DaemonClient {});

    Ok(())
}

async fn handle_event(event_packet: DSEventPacket, addr: SocketAddr, channel_map: ChannelMap) -> Result<(), String> {
    let mut clients = channel_map.write().map_err(|_| "channel_map has been poisoned")?;
    let _client = clients.get_mut(&addr).ok_or("Client not found in channel_map")?;

    debug!("Event: {:#?}", event_packet);

    Ok(())
}
