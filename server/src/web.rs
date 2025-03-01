use std::{borrow::Borrow, collections::{HashMap, HashSet}, fmt::Write, net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use josekit::jwe::alg::rsaes::RsaesJweEncrypter;
use openssl::rand::rand_bytes;
use packet::{events::{EventData, EventType, NodeStatusEvent}, server_daemon::listen::SDListenPacket, server_web::{auth_response::SWAuthResponsePacket, event::SWEventPacket, handshake_request::SWHandshakeRequestPacket}, web_server::{auth::WSAuthPacket, handshake_response::WSHandshakeResponsePacket, listen::WSListenPacket}, Packet, ID};
use sqlx::types::Uuid;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use crate::{db, server::Server, statics::{CONFIG, DAEMON_CHANNEL_MAP, DAEMON_ID_MAP, DAEMON_LISTEN_MAP, DECRYPTER, WEB_CHANNEL_MAP, WEB_KEY_CACHE, WEB_LISTEN_MAP}, types::Tx};

pub struct WebServer;

pub struct WebHandshake {
    user_id: u32,
    pub encrypter: RsaesJweEncrypter,
    challenge: String,
}

pub struct WebSocket {
    pub tx: Tx,
    pub handshake: Option<WebHandshake>,
}

struct PublicKeyQuery {
    user_public_key: String,
}

impl WebServer {
    pub fn new() -> Self {
        Self
    }

    async fn update_listens_for_daemon(&self, addr: &SocketAddr, uuid: &Uuid) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        let daemon_channel_map = DAEMON_CHANNEL_MAP.borrow();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
        let socket = daemon_channel_map.get(addr).ok_or("Daemon not found in DaemonChannelMap")?;

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
        let daemon_listen_map = DAEMON_LISTEN_MAP.borrow();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());
        let events = daemon_listen_map.get(uuid).ok_or("Daemon not found in DaemonListenMap")?.keys().copied().collect::<Vec<_>>();

        socket.tx.unbounded_send(Message::Text(self.encrypt_packet(SDListenPacket {
            events
        }.to_packet()?, &socket.handshake.as_ref().ok_or("Daemon hasn't requested authentication!")?.encrypter)?)).map_err(|_| "Failed to send packet")?;

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_LISTEN_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());
        Ok(())
    }

    async fn query_user_public_key(&self, user_id: u32) -> Result<Arc<Vec<u8>>, String> {
        {
            let cache = WEB_KEY_CACHE.borrow();
            if let Some(v) = cache.get(&user_id) {
                return Ok(v.clone());
            }
        }

        let res = sqlx::query_as!(PublicKeyQuery, "SELECT user_public_key FROM aesterisk.users WHERE user_id = $1", user_id as i32).fetch_one(db::get()).await.map_err(|_| format!("User with ID {} does not exist", user_id))?;

        let cache = WEB_KEY_CACHE.borrow();
        cache.insert(user_id, Arc::new(res.user_public_key.into_bytes()));
        Ok(cache.get(&user_id).ok_or("key should be in cache")?.clone())
    }

    async fn handle_auth(&self, auth_packet: WSAuthPacket, addr: SocketAddr) -> Result<(), String> {
        let mut challenge_bytes = [0; 256];
        rand_bytes(&mut challenge_bytes).map_err(|_| "Could not generate challenge")?;
        let challenge = challenge_bytes.iter().fold(String::new(), |mut s, byte| {
            write!(s, "{:02X}", byte).expect("could not write byte");
            s
        });

        let key = self.query_user_public_key(auth_packet.user_id).await?;

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
        let clients = WEB_CHANNEL_MAP.borrow();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());
        let mut client = clients.get_mut(&addr).ok_or("Client not found in channel_map")?;

        client.handshake = Some(WebHandshake {
            user_id: auth_packet.user_id,
            encrypter: josekit::jwe::RSA_OAEP.encrypter_from_pem(key.as_ref()).map_err(|_| "key should be valid")?,
            challenge: challenge.clone(),
        });

        client.tx.unbounded_send(
            Message::text(
                self.encrypt_packet(
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

    async fn handle_handshake_response(&self, handshake_reponse_packet: WSHandshakeResponsePacket, addr: SocketAddr) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
        let clients = WEB_CHANNEL_MAP.borrow();
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
                self.encrypt_packet(
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

    async fn handle_listen(&self, listen_packet: WSListenPacket, addr: SocketAddr) -> Result<(), String> {
        // debug!("Handling listen packet: {:#?}", listen_packet);

        let mut update_daemons = HashSet::new();
        let mut offline_daemons = HashSet::new();

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_ID_MAP", file!(), line!());
        let daemon_id_map = DAEMON_ID_MAP.borrow();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_ID_MAP", file!(), line!());

        {
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] awaiting WEB_LISTEN_MAP", file!(), line!());
            let web_listen_map = WEB_LISTEN_MAP.borrow();
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] got WEB_LISTEN_MAP", file!(), line!());
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
            let daemon_listen_map = DAEMON_LISTEN_MAP.borrow();
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());

            for event in listen_packet.events.into_iter() {
                for daemon in event.daemons.iter() {
                    update_daemons.insert(*daemon);

                    if let Some(mut listen_map) = daemon_listen_map.get_mut(daemon) {
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

                if let Some(mut listen_map) = web_listen_map.get_mut(&addr) {
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
            self.send_event_from_server(&daemon, EventData::NodeStatus(NodeStatusEvent {
                online: false,
                stats: None,
            })).await?;
        }

        for daemon in update_daemons.into_iter() {
            if let Some(daemon_addr) = daemon_id_map.get(&daemon) {
                self.update_listens_for_daemon(&daemon_addr, &daemon).await?;
            }
        }

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_ID_MAP", file!(), line!());

        Ok(())
    }

    pub async fn send_event_from_server(&self, uuid: &Uuid, event: EventData) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
        let map = DAEMON_LISTEN_MAP.borrow();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());
        let daemon = map.get(uuid).ok_or("Daemon not found in DaemonListenMap")?;
        let clients = daemon.get(&event.event_type());

        if let Some(clients) = clients {
            for client in clients.iter() {
                #[cfg(feature = "lock_debug")]
                debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
                let map = WEB_CHANNEL_MAP.borrow();
                #[cfg(feature = "lock_debug")]
                debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());
                let socket = map.get(client).ok_or("Disconnected client still in WebChannelMap")?;

                socket.tx.unbounded_send(Message::Text(self.encrypt_packet(SWEventPacket {
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
}

#[async_trait]
impl Server for WebServer {
    fn get_bind_addr(&self) ->  &'static str {
        &CONFIG.sockets.web
    }

    fn get_tracing_name(&self) -> &'static str {
        "web"
    }

    fn get_issuer(&self) ->  &'static str {
        "aesterisk/web"
    }

    fn get_decrypter(&self) ->  &'static josekit::jwe::alg::rsaes::RsaesJweDecrypter {
        &DECRYPTER
    }

    async fn on_accept(&self, addr: SocketAddr, tx: Tx) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());

        WEB_CHANNEL_MAP.insert(addr, WebSocket {
            tx,
            handshake: None,
        });

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped WEB_CHANNEL_MAP", file!(), line!());

        Ok(())
    }

    async fn on_disconnect(&self, addr: SocketAddr) -> Result<(), String> {
        let mut update_daemons = HashSet::new();

        {
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] awaiting WEB_LISTEN_MAP", file!(), line!());
            let web_listen_map = WEB_LISTEN_MAP.borrow();
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] got WEB_LISTEN_MAP", file!(), line!());
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
            let daemon_listen_map = DAEMON_LISTEN_MAP.borrow();
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
            let web_channel_map = WEB_CHANNEL_MAP.borrow();
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());

            web_channel_map.remove(&addr);
            if let Some(listen_map) = web_listen_map.get(&addr) {
                for (event, daemons) in listen_map.iter() {
                    for daemon in daemons.iter() {
                        update_daemons.insert(*daemon);

                        let mut listen_map = daemon_listen_map.get_mut(daemon).ok_or("daemon not found in DaemonListenMap")?;
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
            if let Some(daemon_addr) = DAEMON_ID_MAP.get(&daemon) {
                self.update_listens_for_daemon(&daemon_addr, &daemon).await?;
            }
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] got DAEMON_ID_MAP", file!(), line!());
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] dropped DAEMON_ID_MAP", file!(), line!());
        }

        Ok(())
    }

    async fn on_decrypt_error(&self, addr: SocketAddr) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
        WEB_CHANNEL_MAP.get(&addr).ok_or("Client not found in channel_map")?.tx.close_channel();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped WEB_CHANNEL_MAP", file!(), line!());

        Ok(())
    }

    async fn on_packet(&self, packet: Packet, addr: SocketAddr) -> Result<(), String> {
        match packet.id {
            ID::WSAuth => {
                self.handle_auth(WSAuthPacket::parse(packet).expect("WSAuthPacket should be Some"), addr).await
            },
            ID::WSHandshakeResponse => {
                self.handle_handshake_response(WSHandshakeResponsePacket::parse(packet).expect("WSHandshakeResponsePacket should be Some"), addr).await
            }
            ID::WSListen => {
                self.handle_listen(WSListenPacket::parse(packet).expect("WSListenPacket should be Some"), addr).await
            },
            _ => {
                Err(format!("Should not receive [SD]* packet: {:?}", packet.id))
            },
        }
    }
}
