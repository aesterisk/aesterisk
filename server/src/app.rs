use std::{collections::HashMap, net::SocketAddr, sync::{Arc, Mutex, RwLock}, time::{Duration, SystemTime}};

use futures_channel::mpsc::{self, unbounded};
use futures_util::{future, pin_mut, stream::{SplitSink, SplitStream}, FutureExt, StreamExt, TryStreamExt};
use josekit::{jwe::{alg::rsaes::{RsaesJweDecrypter, RsaesJweEncrypter}, JweHeader}, jwt::{self, JwtPayload, JwtPayloadValidator}};
use openssl::rand::rand_bytes;
use sqlx::PgPool;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite::{self, Message}, WebSocketStream};

use packet::{app_server::{auth::ASAuthPacket, handshake_response::ASHandshakeResponsePacket, listen::ASListenPacket}, server_app::{auth_response::SAAuthResponsePacket, handshake_request::SAHandshakeRequestPacket}, ListenEvent, Packet, ID};
use tracing::{debug, error, info, warn};

use crate::{CONFIG, DECRYPTER};

struct AppClient {
    listens: Vec<ListenEvent>,
}

struct AppHandshake {
    user_id: u32,
    encrypter: RsaesJweEncrypter,
    challenge: String,
}

struct AppSocket {
    tx: Tx,
    handshake: Option<AppHandshake>,
    authed: Option<AppClient>,
}

type Tx = mpsc::UnboundedSender<Message>;
type Rx = mpsc::UnboundedReceiver<Message>;
type ChannelMap = Arc<RwLock<HashMap<SocketAddr, AppSocket>>>;
type KeyCache = Arc<Mutex<HashMap<u32, Arc<Vec<u8>>>>>;

pub async fn start(pool: PgPool) {
    let try_socket = TcpListener::bind(&CONFIG.sockets.app).await;
    let listener = match try_socket {
        Ok(listener) => listener,
        Err(e) => {
            error!("Error binding to socket: {}", e);
            return;
        }
    };

    info!("Listening on: {}", &CONFIG.sockets.app);

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

#[tracing::instrument(name = "app", skip(raw_stream, channel_map, key_cache, pool), fields(%addr))]
async fn accept_connection(raw_stream: TcpStream, addr: SocketAddr, channel_map: ChannelMap, key_cache: KeyCache, pool: PgPool) -> Result<(), String> {
    info!("Accepted TCP connection");

    let stream = tokio_tungstenite::accept_async(raw_stream).await.map_err(|e| format!("Could not accept connection: {}", error_to_string(e)))?;

    let (write, read) = stream.split();

    let (tx, rx) = unbounded();
    channel_map.write().map_err(|_| "channel_map has been poisoned")?.insert(addr, AppSocket {
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

#[tracing::instrument(name = "app", skip(msg, channel_map, key_cache, pool), fields(%addr))]
async fn handle_packet(msg: String, addr: SocketAddr, channel_map: ChannelMap, key_cache: KeyCache, pool: PgPool) -> Result<(), String> {
    let packet = decrypt_packet(&msg, &DECRYPTER)?;

    match packet.id {
        ID::ASAuth => {
            handle_auth(ASAuthPacket::parse(packet).expect("ASAuthPacket should be Some"), addr, channel_map, key_cache, pool).await
        },
        ID::ASHandshakeResponse => {
            handle_handshake_response(ASHandshakeResponsePacket::parse(packet).expect("ASHandshakeResponsePacket should be Some"), addr, channel_map).await
        }
        ID::ASListen => {
            handle_listen(ASListenPacket::parse(packet).expect("ASListenPacket should be Some"), addr, channel_map).await
        },
        _ => {
            Err(format!("Should not receive [SD]* packet: {:?}", packet.id))
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

fn decrypt_packet(msg: &str, decrypter: &RsaesJweDecrypter) -> Result<Packet, String> {
    let (payload, _) = jwt::decode_with_decrypter(msg, decrypter).map_err(|_| "should decrypt")?;

    let mut validator = JwtPayloadValidator::new();
    validator.set_issuer("aesterisk/app");
    validator.set_base_time(SystemTime::now());
    validator.set_min_issued_time(SystemTime::now() - Duration::from_secs(60));
    validator.set_max_issued_time(SystemTime::now());

    validator.validate(&payload).map_err(|_| "invalid token")?;

    // TODO: maybe don't clone hehe
    let try_packet = Packet::from_value(payload.claim("p").ok_or("should have .p")?.clone());

    Ok(try_packet.ok_or(format!("Could not parse packet: \"{}\"", msg))?)
}

struct PublicKeyQuery {
    user_public_key: String,
}

async fn query_user_public_key(user_id: u32, key_cache: KeyCache, pool: PgPool) -> Result<Arc<Vec<u8>>, String> {
    {
        let cache = key_cache.lock().map_err(|_| "key_cache has been poisoned")?;
        if let Some(v) = cache.get(&user_id) {
            return Ok(v.clone());
        }
    }

    let res = sqlx::query_as!(PublicKeyQuery, "SELECT user_public_key FROM aesterisk.users WHERE user_id = $1", user_id as i32).fetch_one(&pool).await.map_err(|_| format!("User with ID {} does not exist", user_id))?;

    let mut cache = key_cache.lock().map_err(|_| "key_cache has been poisoned")?;
    cache.insert(user_id, Arc::new(res.user_public_key.into_bytes()));
    Ok(cache.get(&user_id).ok_or("key should be in cache")?.clone())
}

async fn handle_auth(auth_packet: ASAuthPacket, addr: SocketAddr, channel_map: ChannelMap, key_cache: KeyCache, pool: PgPool) -> Result<(), String> {
    let mut challenge_bytes = [0; 256];
    rand_bytes(&mut challenge_bytes).map_err(|_| "Could not generate challenge")?;
    let challenge = challenge_bytes.iter().map(|byte| format!("{:02X}", byte)).collect::<String>();

    let key = query_user_public_key(auth_packet.user_id, key_cache, pool).await?;

    let mut clients = channel_map.write().map_err(|_| "channel_map has been poisoned")?;
    let client = clients.get_mut(&addr).ok_or("Client not found in channel_map")?;

    client.handshake = Some(AppHandshake {
        user_id: auth_packet.user_id,
        encrypter: josekit::jwe::RSA_OAEP.encrypter_from_pem(key.as_ref()).map_err(|_| "key should be valid")?,
        challenge: challenge.clone(),
    });

    client.tx.unbounded_send(
        Message::text(
            encrypt_packet(
                SAHandshakeRequestPacket {
                    challenge
                }.to_packet(),
                &client.handshake.as_ref().ok_or("Client hasn't requested authentication")?.encrypter,
            )?
        )
    ).map_err(|_| "Failed to send packet")?;

    Ok(())
}

async fn handle_handshake_response(handshake_reponse_packet: ASHandshakeResponsePacket, addr: SocketAddr, channel_map: ChannelMap) -> Result<(), String> {
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
                SAAuthResponsePacket {
                    success: true,
                }.to_packet(),
                &client.handshake.as_ref().ok_or("Client hasn't requested authentication")?.encrypter,
            )?
        )
    ).map_err(|_| "Failed to send packet")?;

    client.authed = Some(AppClient {
        listens: Vec::new(),
    });

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
