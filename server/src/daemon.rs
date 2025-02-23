use std::{net::SocketAddr, sync::Arc, time::{Duration, SystemTime}, fmt::Write};

use futures_channel::mpsc::unbounded;
use futures_util::{future, pin_mut, stream::{SplitSink, SplitStream}, FutureExt, StreamExt, TryStreamExt};
use josekit::{jwe::{alg::rsaes::{RsaesJweDecrypter, RsaesJweEncrypter}, JweHeader}, jwt::{self, JwtPayload, JwtPayloadValidator}};
use openssl::rand::rand_bytes;
use sqlx::{types::Uuid, PgPool};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite::{self, Message}, WebSocketStream};

use packet::{daemon_server::{auth::DSAuthPacket, event::DSEventPacket, handshake_response::DSHandshakeResponsePacket}, events::{Event, EventData, NodeStatusEvent}, server_daemon::{auth_response::SDAuthResponsePacket, handshake_request::SDHandshakeRequestPacket, listen::SDListenPacket}, server_web::event::SWEventPacket, Packet, ID};
#[allow(unused_imports)]
use tracing::{debug, error, info, warn};

use crate::{Rx, Tx, CONFIG, DAEMON_CHANNEL_MAP, DAEMON_ID_MAP, DAEMON_KEY_CACHE, DAEMON_LISTEN_MAP, DECRYPTER, WEB_CHANNEL_MAP};

pub struct DaemonHandshake {
    daemon_uuid: Uuid,
    pub encrypter: RsaesJweEncrypter,
    challenge: String,
}

pub struct DaemonSocket {
    pub tx: Tx,
    pub handshake: Option<DaemonHandshake>,
}

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

    loop {
        let conn = listener.accept().await;

        match conn {
            Ok((stream, addr)) => {
                tokio::spawn(accept_connection(stream, addr, pool.clone()).then(|res| match res {
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
        tungstenite::Error::Utf8 => "Error in UTF-8 encoding".into(),
        tungstenite::Error::Io(e) => format!("IO error ({})", e.kind()),
        tungstenite::Error::Tls(_) => "TLS error".into(),
        tungstenite::Error::Url(_) => "Invalid URL".into(),
        tungstenite::Error::Http(_) => "HTTP error".into(),
        tungstenite::Error::HttpFormat(_) => "HTTP format error".into(),
        tungstenite::Error::Capacity(_) => "Buffer capacity exhausted".into(),
        tungstenite::Error::Protocol(_) => "Protocol violation".into(),
        tungstenite::Error::AlreadyClosed => "Connection already closed".into(),
        tungstenite::Error::AttackAttempt => "Attack attempt detected".into(),
        tungstenite::Error::WriteBufferFull(_) => "Write buffer full".into(),
        tungstenite::Error::ConnectionClosed => "Connection closed".into(),
    }
}

#[tracing::instrument(name = "daemon", skip(raw_stream, pool), fields(%addr))]
async fn accept_connection(raw_stream: TcpStream, addr: SocketAddr, pool: PgPool) -> Result<(), String> {
    info!("Accepted TCP connection");

    let stream = tokio_tungstenite::accept_async(raw_stream).await.map_err(|e| format!("Could not accept connection: {}", error_to_string(e)))?;

    let (write, read) = stream.split();

    let (tx, rx) = unbounded();
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
    DAEMON_CHANNEL_MAP.write().await.insert(addr, DaemonSocket {
        tx,
        handshake: None,
    });
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());

    handle_client(write, read, addr, rx, pool).await?;

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());
    Ok(())
}


async fn handle_client(write: SplitSink<WebSocketStream<TcpStream>, Message>, read: SplitStream<WebSocketStream<TcpStream>>, addr: SocketAddr, rx: Rx, pool: PgPool) -> Result<(), String> {
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

        tokio::spawn(handle_packet(text, addr, pool.clone()).then(|res| match res {
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

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
    let uuid = DAEMON_CHANNEL_MAP.read().await.get(&addr).ok_or("Daemon not found in DaemonChannelMap")?.handshake.as_ref().ok_or("Daemon hasn't authenticated")?.daemon_uuid;
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
    DAEMON_CHANNEL_MAP.write().await.remove(&addr);
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting DAEMON_ID_MAP", file!(), line!());
    DAEMON_ID_MAP.write().await.remove(&uuid);
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got DAEMON_ID_MAP", file!(), line!());
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped DAEMON_ID_MAP", file!(), line!());
    send_event_from_server(&uuid, EventData::NodeStatus(NodeStatusEvent {
        online: false,
        stats: None,
    })).await?;

    info!("Disconnected");

    Ok(())
}

#[tracing::instrument(name = "daemon", skip(msg, pool), fields(%addr))]
async fn handle_packet(msg: String, addr: SocketAddr, pool: PgPool) -> Result<(), String> {
    let packet = decrypt_packet(&msg, &DECRYPTER, &addr).await?;

    match packet.id {
        ID::DSAuth => {
            handle_auth(DSAuthPacket::parse(packet).expect("DSAuthPacket should be Some"), addr, pool).await
        },
        ID::DSHandshakeResponse => {
            handle_handshake_response(DSHandshakeResponsePacket::parse(packet).expect("DSHandshakeResponsePacket should be Some"), addr).await
        }
        ID::DSEvent => {
            handle_event(DSEventPacket::parse(packet).expect("DSEventPacket should be Some"), addr).await
        },
        _ => {
            Err(format!("Should not receive [SW]* packet: {:?}", packet.id))
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

async fn decrypt_packet(msg: &str, decrypter: &RsaesJweDecrypter, addr: &SocketAddr) -> Result<Packet, String> {
    let (payload, _) = jwt::decode_with_decrypter(msg, decrypter).expect("should decrypt");

    let mut validator = JwtPayloadValidator::new();
    validator.set_issuer("aesterisk/daemon");
    validator.set_base_time(SystemTime::now());
    validator.set_min_issued_time(SystemTime::now() - Duration::from_secs(60));
    validator.set_max_issued_time(SystemTime::now());

    match validator.validate(&payload) {
        Ok(()) => (),
        Err(e) => {
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
            DAEMON_CHANNEL_MAP.write().await.get(addr).ok_or("Client not found in channel_map")?.tx.close_channel();
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());
            return Err(format!("Invalid token: {}", e));
        }
    }

    // TODO: maybe don't clone hehe
    let try_packet = Packet::from_value(payload.claim("p").expect("should have .p").clone());

    try_packet.ok_or(format!("Could not parse packet: \"{}\"", msg))
}

struct PublicKeyQuery {
    node_public_key: String,
}

async fn query_user_public_key(daemon_uuid: &Uuid, pool: PgPool) -> Result<Arc<Vec<u8>>, String> {
    {
        let cache = DAEMON_KEY_CACHE.lock().await;
        if let Some(v) = cache.get(daemon_uuid) {
            return Ok(v.clone());
        }
    }

    let res = sqlx::query_as!(PublicKeyQuery, "SELECT node_public_key FROM aesterisk.nodes WHERE node_uuid = $1", daemon_uuid).fetch_one(&pool).await.map_err(|_| format!("Node with UUID {} does not exist", &daemon_uuid))?;

    let mut cache = DAEMON_KEY_CACHE.lock().await;
    cache.insert(*daemon_uuid, Arc::new(res.node_public_key.into_bytes()));
    Ok(cache.get(daemon_uuid).ok_or("key should be in cache")?.clone())
}

async fn handle_auth(auth_packet: DSAuthPacket, addr: SocketAddr, pool: PgPool) -> Result<(), String> {
    let mut challenge_bytes = [0; 256];
    rand_bytes(&mut challenge_bytes).map_err(|_| "Could not generate challenge")?;
    let challenge = challenge_bytes.iter().fold(String::new(), |mut s, byte| {
        write!(s, "{:02X}", byte).expect("could not write byte");
        s
    });

    let uuid = Uuid::parse_str(&auth_packet.daemon_uuid).map_err(|_| "Could not parse UUID")?;
    let key = query_user_public_key(&uuid, pool).await?;

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
    let mut clients = DAEMON_CHANNEL_MAP.write().await;
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
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

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());
    Ok(())
}

async fn handle_handshake_response(handshake_reponse_packet: DSHandshakeResponsePacket, addr: SocketAddr) -> Result<(), String> {
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
    let mut clients = DAEMON_CHANNEL_MAP.write().await;
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
    let client = clients.get_mut(&addr).ok_or("Client not found in channel_map")?;

    if handshake_reponse_packet.challenge != client.handshake.as_ref().ok_or("Client hasn't requested authentication")?.challenge {
        warn!("Failed authentication");
        client.tx.close_channel();
        return Err("Challenge does not match".to_string());
    }

    info!("Authenticated");

    let uuid = client.handshake.as_ref().ok_or("Client hasn't requested authentication")?.daemon_uuid;
    let encrypter = &client.handshake.as_ref().ok_or("Client hasn't requested authentication")?.encrypter;

    client.tx.unbounded_send(
        Message::text(
            encrypt_packet(
                SDAuthResponsePacket {
                    success: true,
                }.to_packet()?,
                encrypter,
            )?
        )
    ).map_err(|_| "Failed to send packet")?;

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
    let daemon_listen_map = DAEMON_LISTEN_MAP.read().await;
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());
    if let Some(listen_map) = daemon_listen_map.get(&uuid) {
        let events = listen_map.keys().copied().collect::<Vec<_>>();

        client.tx.unbounded_send(Message::Text(encrypt_packet(SDListenPacket {
            events
        }.to_packet()?, encrypter)?)).map_err(|_| "Failed to send packet")?;
    }

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting DAEMON_ID_MAP", file!(), line!());
    DAEMON_ID_MAP.write().await.insert(uuid, addr);
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got DAEMON_ID_MAP", file!(), line!());
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped DAEMON_ID_MAP", file!(), line!());

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped DAEMON_LISTEN_MAP", file!(), line!());
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());
    Ok(())
}

async fn handle_event(event_packet: DSEventPacket, addr: SocketAddr) -> Result<(), String> {
    // debug!("Event: {:#?}", event_packet);

    send_event_from_daemon(&addr, event_packet.data).await?;

    Ok(())
}

pub async fn send_event_from_server(uuid: &Uuid, event: EventData) -> Result<(), String> {
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
    let map = DAEMON_LISTEN_MAP.read().await;
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());
    let clients = map.get(uuid).ok_or("Daemon not found in DaemonListenMap")?.get(&event.event_type());

    if let Some(clients) = clients {
        for client in clients.iter() {
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
            let map = WEB_CHANNEL_MAP.read().await;
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());
            let socket = map.get(client).ok_or("Disconnected client still in WebChannelMap")?;

            socket.tx.unbounded_send(Message::Text(encrypt_packet(SWEventPacket {
                event: event.clone(),
                daemon: *uuid,
            }.to_packet()?, &socket.handshake.as_ref().ok_or("Client hasn't requested authentication")?.encrypter)?)).map_err(|_| "Could not send packet to client")?;

            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] dropped WEB_CHANNEL_MAP", file!(), line!());
        }
    }

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped DAEMON_LISTEN_MAP", file!(), line!());
    Ok(())
}

async fn send_event_from_daemon(addr: &SocketAddr, event: EventData) -> Result<(), String> {
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
    let uuid = DAEMON_CHANNEL_MAP.read().await.get(addr).ok_or("Daemon not found in DaemonChannelMap")?.handshake.as_ref().ok_or("Client hasn't requested authentication")?.daemon_uuid;
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());
    send_event_from_server(&uuid, event).await
}
