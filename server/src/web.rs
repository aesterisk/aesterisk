use std::{collections::{HashMap, HashSet}, net::SocketAddr, sync::Arc, time::{Duration, SystemTime}, fmt::Write};

use futures_channel::mpsc::unbounded;
use futures_util::{future, pin_mut, stream::{SplitSink, SplitStream}, FutureExt, StreamExt, TryStreamExt};
use josekit::{jwe::{alg::rsaes::{RsaesJweDecrypter, RsaesJweEncrypter}, JweHeader}, jwt::{self, JwtPayload, JwtPayloadValidator}};
use openssl::rand::rand_bytes;
use sqlx::{types::Uuid, PgPool};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite::{self, Message}, WebSocketStream};

use packet::{events::{EventData, EventType, NodeStatusEvent}, server_daemon::listen::SDListenPacket, server_web::{auth_response::SWAuthResponsePacket, handshake_request::SWHandshakeRequestPacket}, web_server::{auth::WSAuthPacket, handshake_response::WSHandshakeResponsePacket, listen::WSListenPacket}, Packet, ID};
#[allow(unused_imports)]
use tracing::{debug, error, info, warn};

use crate::{daemon::send_event_from_server, Rx, Tx, CONFIG, DAEMON_CHANNEL_MAP, DAEMON_ID_MAP, DAEMON_LISTEN_MAP, DECRYPTER, WEB_CHANNEL_MAP, WEB_KEY_CACHE, WEB_LISTEN_MAP};

pub struct WebHandshake {
    user_id: u32,
    pub encrypter: RsaesJweEncrypter,
    challenge: String,
}

pub struct WebSocket {
    pub tx: Tx,
    pub handshake: Option<WebHandshake>,
}

pub async fn start(pool: PgPool) {
    let try_socket = TcpListener::bind(&CONFIG.sockets.web).await;
    let listener = match try_socket {
        Ok(listener) => listener,
        Err(e) => {
            error!("Error binding to socket: {}", e);
            return;
        }
    };

    info!("Listening on: {}", &CONFIG.sockets.web);

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

#[tracing::instrument(name = "web", skip(raw_stream, pool), fields(%addr))]
async fn accept_connection(raw_stream: TcpStream, addr: SocketAddr, pool: PgPool) -> Result<(), String> {
    info!("Accepted TCP connection");

    let stream = tokio_tungstenite::accept_async(raw_stream).await.map_err(|e| format!("Could not accept connection: {}", error_to_string(e)))?;

    let (write, read) = stream.split();

    let (tx, rx) = unbounded();
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
    WEB_CHANNEL_MAP.write().await.insert(addr, WebSocket {
        tx,
        handshake: None,
    });

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());

    handle_client(write, read, addr, rx, pool).await?;

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped WEB_CHANNEL_MAP", file!(), line!());

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

    let mut update_daemons = HashSet::new();

    {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting WEB_LISTEN_MAP", file!(), line!());
        let web_listen_map = WEB_LISTEN_MAP.read().await;
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got WEB_LISTEN_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
        let mut daemon_listen_map = DAEMON_LISTEN_MAP.write().await;
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
        let mut web_channel_map = WEB_CHANNEL_MAP.write().await;
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());

        web_channel_map.remove(&addr);
        if let Some(listen_map) = web_listen_map.get(&addr) {
            for (event, daemons) in listen_map.iter() {
                for daemon in daemons.iter() {
                    update_daemons.insert(*daemon);

                    let listen_map = daemon_listen_map.get_mut(daemon).ok_or("daemon not found in DaemonListenMap")?;
                    let event_map = listen_map.get_mut(event).ok_or("event not found in DaemonListenMap")?;

                    event_map.remove(&addr);

                    if event_map.is_empty() {
                        listen_map.remove(event);
                    }
                }
            }
        }

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped WEB_CHANNEL_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_LISTEN_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped WEB_LISTEN_MAP", file!(), line!());
    }

    for daemon in update_daemons {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_ID_MAP", file!(), line!());
        if let Some(daemon_addr) = DAEMON_ID_MAP.read().await.get(&daemon) {
            update_listens_for_daemon(daemon_addr, &daemon).await?;
        }
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_ID_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_ID_MAP", file!(), line!());
    }

    info!("Disconnected");

    Ok(())
}

#[tracing::instrument(name = "web", skip(msg, pool), fields(%addr))]
async fn handle_packet(msg: String, addr: SocketAddr, pool: PgPool) -> Result<(), String> {
    let packet = decrypt_packet(&msg, &DECRYPTER)?;

    match packet.id {
        ID::WSAuth => {
            handle_auth(WSAuthPacket::parse(packet).expect("WSAuthPacket should be Some"), addr, pool).await
        },
        ID::WSHandshakeResponse => {
            handle_handshake_response(WSHandshakeResponsePacket::parse(packet).expect("WSHandshakeResponsePacket should be Some"), addr).await
        }
        ID::WSListen => {
            handle_listen(WSListenPacket::parse(packet).expect("WSListenPacket should be Some"), addr).await
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
    validator.set_issuer("aesterisk/web");
    validator.set_base_time(SystemTime::now());
    validator.set_min_issued_time(SystemTime::now() - Duration::from_secs(60));
    validator.set_max_issued_time(SystemTime::now());

    validator.validate(&payload).map_err(|_| "invalid token")?;

    // TODO: maybe don't clone hehe
    let try_packet = Packet::from_value(payload.claim("p").ok_or("should have .p")?.clone());

    try_packet.ok_or(format!("Could not parse packet: \"{}\"", msg))
}

struct PublicKeyQuery {
    user_public_key: String,
}

async fn query_user_public_key(user_id: u32, pool: PgPool) -> Result<Arc<Vec<u8>>, String> {
    {
        let cache = WEB_KEY_CACHE.lock().await;
        if let Some(v) = cache.get(&user_id) {
            return Ok(v.clone());
        }
    }

    let res = sqlx::query_as!(PublicKeyQuery, "SELECT user_public_key FROM aesterisk.users WHERE user_id = $1", user_id as i32).fetch_one(&pool).await.map_err(|_| format!("User with ID {} does not exist", user_id))?;

    let mut cache = WEB_KEY_CACHE.lock().await;
    cache.insert(user_id, Arc::new(res.user_public_key.into_bytes()));
    Ok(cache.get(&user_id).ok_or("key should be in cache")?.clone())
}

async fn handle_auth(auth_packet: WSAuthPacket, addr: SocketAddr, pool: PgPool) -> Result<(), String> {
    let mut challenge_bytes = [0; 256];
    rand_bytes(&mut challenge_bytes).map_err(|_| "Could not generate challenge")?;
    let challenge = challenge_bytes.iter().fold(String::new(), |mut s, byte| {
        write!(s, "{:02X}", byte).expect("could not write byte");
        s
    });

    let key = query_user_public_key(auth_packet.user_id, pool).await?;

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
    let mut clients = WEB_CHANNEL_MAP.write().await;
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());
    let client = clients.get_mut(&addr).ok_or("Client not found in channel_map")?;

    client.handshake = Some(WebHandshake {
        user_id: auth_packet.user_id,
        encrypter: josekit::jwe::RSA_OAEP.encrypter_from_pem(key.as_ref()).map_err(|_| "key should be valid")?,
        challenge: challenge.clone(),
    });

    client.tx.unbounded_send(
        Message::text(
            encrypt_packet(
                SWHandshakeRequestPacket {
                    challenge
                }.to_packet()?,
                &client.handshake.as_ref().ok_or("Client hasn't requested authentication")?.encrypter,
            )?
        )
    ).map_err(|_| "Failed to send packet")?;

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped WEB_CHANNEL_MAP", file!(), line!());
    Ok(())
}

async fn handle_handshake_response(handshake_reponse_packet: WSHandshakeResponsePacket, addr: SocketAddr) -> Result<(), String> {
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
    let mut clients = WEB_CHANNEL_MAP.write().await;
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());
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
                SWAuthResponsePacket {
                    success: true,
                }.to_packet()?,
                &client.handshake.as_ref().ok_or("Client hasn't requested authentication")?.encrypter,
            )?
        )
    ).map_err(|_| "Failed to send packet")?;

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped WEB_CHANNEL_MAP", file!(), line!());
    Ok(())
}

async fn handle_listen(listen_packet: WSListenPacket, addr: SocketAddr) -> Result<(), String> {
    // debug!("Handling listen packet: {:#?}", listen_packet);

    let mut update_daemons = HashSet::new();
    let mut offline_daemons = HashSet::new();

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting DAEMON_ID_MAP", file!(), line!());
    let daemon_id_map = DAEMON_ID_MAP.read().await;
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got DAEMON_ID_MAP", file!(), line!());

    {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting WEB_LISTEN_MAP", file!(), line!());
        let mut web_listen_map = WEB_LISTEN_MAP.write().await;
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got WEB_LISTEN_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
        let mut daemon_listen_map = DAEMON_LISTEN_MAP.write().await;
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());

        for event in listen_packet.events.into_iter() {
            for daemon in event.daemons.iter() {
                update_daemons.insert(*daemon);

                if let Some(listen_map) = daemon_listen_map.get_mut(daemon) {
                    if let Some(client_set) = listen_map.get_mut(&event.event) {
                        client_set.insert(addr);
                    } else {
                        listen_map.insert(event.event, HashSet::from([addr]));
                    }
                } else {
                    let mut set = HashSet::new();
                    set.insert(addr);
                    let mut listen_map = HashMap::new();
                    listen_map.insert(event.event, set);
                    daemon_listen_map.insert(*daemon, listen_map);
                }

                if event.event == EventType::NodeStatus && daemon_id_map.get(daemon).is_none() {
                    offline_daemons.insert(*daemon);
                }
            }

            if let Some(listen_map) = web_listen_map.get_mut(&addr) {
                if let Some(daemon_set) = listen_map.get_mut(&event.event) {
                    for daemon in event.daemons.iter() {
                        daemon_set.insert(*daemon);
                    }
                } else {
                    listen_map.insert(event.event, HashSet::from_iter(event.daemons.into_iter()));
                }
            } else {
                web_listen_map.insert(addr, HashMap::from([(event.event, HashSet::from_iter(event.daemons.into_iter()))]));
            }
        }

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_LISTEN_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped WEB_LISTEN_MAP", file!(), line!());
    }

    for daemon in offline_daemons.into_iter() {
        send_event_from_server(&daemon, EventData::NodeStatus(NodeStatusEvent {
            online: false,
            stats: None,
        })).await?;
    }

    for daemon in update_daemons.into_iter() {
        if let Some(daemon_addr) = daemon_id_map.get(&daemon) {
            update_listens_for_daemon(daemon_addr, &daemon).await?;
        }
    }

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped DAEMON_ID_MAP", file!(), line!());

    Ok(())
}

async fn update_listens_for_daemon(addr: &SocketAddr, uuid: &Uuid) -> Result<(), String> {
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
    let daemon_channel_map = DAEMON_CHANNEL_MAP.read().await;
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
    let socket = daemon_channel_map.get(addr).ok_or("Daemon not found in DaemonChannelMap")?;

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
    let daemon_listen_map = DAEMON_LISTEN_MAP.read().await;
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());
    let events = daemon_listen_map.get(uuid).ok_or("Daemon not found in DaemonListenMap")?.keys().copied().collect::<Vec<_>>();

    socket.tx.unbounded_send(Message::Text(encrypt_packet(SDListenPacket {
        events
    }.to_packet()?, &socket.handshake.as_ref().ok_or("Daemon hasn't requested authentication!")?.encrypter)?)).map_err(|_| "Failed to send packet")?;

    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped DAEMON_LISTEN_MAP", file!(), line!());
    #[cfg(feature = "lock_debug")]
    debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());
    Ok(())
}
