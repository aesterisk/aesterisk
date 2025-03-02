use std::{borrow::Borrow, net::SocketAddr, sync::Arc, fmt::Write};

use async_trait::async_trait;
use josekit::jwe::alg::rsaes::{RsaesJweDecrypter, RsaesJweEncrypter};
use openssl::rand::rand_bytes;
use packet::{daemon_server::{auth::DSAuthPacket, event::DSEventPacket, handshake_response::DSHandshakeResponsePacket}, events::{EventData, NodeStatusEvent}, server_daemon::{auth_response::SDAuthResponsePacket, handshake_request::SDHandshakeRequestPacket, listen::SDListenPacket}, server_web::event::SWEventPacket, Packet, ID};
use sqlx::types::Uuid;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, instrument, warn};

use crate::{db, server::Server, state::State, statics::{CONFIG, DECRYPTER}, types::{DaemonChannelMap, DaemonKeyCache, DaemonListenMap, Tx, WebChannelMap}};

pub struct DaemonHandshake {
    pub daemon_uuid: Uuid,
    pub encrypter: RsaesJweEncrypter,
    pub challenge: String,
}

pub struct DaemonSocket {
    pub tx: Tx,
    pub handshake: Option<DaemonHandshake>,
}

pub struct DaemonServer {
    state: Arc<State>,
}

struct PublicKeyQuery {
    node_public_key: String,
}

impl DaemonServer {
    pub fn new(state: Arc<State>) -> Self {
        Self {
            state
        }
    }

    pub async fn send_event_from_server(&self, uuid: &Uuid, event: EventData) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
        let map: &DaemonListenMap = self.state.daemon_listen_map.borrow();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());
        let daemon = map.get(uuid).ok_or("Daemon not found in DaemonListenMap")?;
        let clients = daemon.get(&event.event_type());

        if let Some(clients) = clients {
            for client in clients.iter() {
                #[cfg(feature = "lock_debug")]
                debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
                let map: &WebChannelMap = self.state.web_channel_map.borrow();
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

    async fn send_event_from_daemon(&self, addr: &SocketAddr, event: EventData) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        let uuid = self.state.daemon_channel_map.get(addr).ok_or("Daemon not found in DaemonChannelMap")?.handshake.as_ref().ok_or("Client hasn't requested authentication")?.daemon_uuid;
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());
        self.send_event_from_server(&uuid, event).await
    }

    async fn query_user_public_key(&self, daemon_uuid: &Uuid) -> Result<Arc<Vec<u8>>, String> {
        {
            let cache: &DaemonKeyCache = self.state.daemon_key_cache.borrow();
            if let Some(v) = cache.get(daemon_uuid) {
                return Ok(v.clone());
            }
        }

        let res = sqlx::query_as!(PublicKeyQuery, "SELECT node_public_key FROM aesterisk.nodes WHERE node_uuid = $1", daemon_uuid).fetch_one(db::get()).await.map_err(|_| format!("Node with UUID {} does not exist", &daemon_uuid))?;

        let cache: &DaemonKeyCache = self.state.daemon_key_cache.borrow();
        cache.insert(*daemon_uuid, Arc::new(res.node_public_key.into_bytes()));
        Ok(cache.get(daemon_uuid).ok_or("key should be in cache")?.clone())
    }

    async fn handle_auth(&self, auth_packet: DSAuthPacket, addr: SocketAddr) -> Result<(), String> {
        let mut challenge_bytes = [0; 256];
        rand_bytes(&mut challenge_bytes).map_err(|_| "Could not generate challenge")?;
        let challenge = challenge_bytes.iter().fold(String::new(), |mut s, byte| {
            write!(s, "{:02X}", byte).expect("could not write byte");
            s
        });

        let uuid = Uuid::parse_str(&auth_packet.daemon_uuid).map_err(|_| "Could not parse UUID")?;
        let key = self.query_user_public_key(&uuid).await?;

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        let clients: &DaemonChannelMap = self.state.daemon_channel_map.borrow();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
        let mut client = clients.get_mut(&addr).ok_or("Client not found in channel_map")?;

        client.handshake = Some(DaemonHandshake {
            daemon_uuid: uuid,
            encrypter: josekit::jwe::RSA_OAEP.encrypter_from_pem(key.as_ref()).map_err(|_| "key should be valid")?,
            challenge: challenge.clone(),
        });

        client.tx.unbounded_send(
            Message::text(
                self.encrypt_packet(
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

    async fn handle_handshake_response(&self, handshake_reponse_packet: DSHandshakeResponsePacket, addr: SocketAddr) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        let clients: &DaemonChannelMap = self.state.daemon_channel_map.borrow();
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
                self.encrypt_packet(
                    SDAuthResponsePacket {
                        success: true,
                    }.to_packet()?,
                    encrypter,
                )?
            )
        ).map_err(|_| "Failed to send packet")?;

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
        let daemon_listen_map: &DaemonListenMap = self.state.daemon_listen_map.borrow();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());
        if let Some(listen_map) = daemon_listen_map.get(&uuid) {
            let events = listen_map.keys().copied().collect::<Vec<_>>();

            client.tx.unbounded_send(Message::Text(self.encrypt_packet(SDListenPacket {
                events
            }.to_packet()?, encrypter)?)).map_err(|_| "Failed to send packet")?;
        }

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_ID_MAP", file!(), line!());
        self.state.daemon_id_map.insert(uuid, addr);
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

    async fn handle_event(&self, event_packet: DSEventPacket, addr: SocketAddr) -> Result<(), String> {
        // debug!("Event: {:#?}", event_packet);

        self.send_event_from_daemon(&addr, event_packet.data).await?;

        Ok(())
    }
}

#[async_trait]
impl Server for DaemonServer {
    fn get_tracing_name(&self) -> &'static str {
        "daemon"
    }

    fn get_bind_addr(&self) -> &'static str {
        &CONFIG.sockets.daemon
    }

    fn get_decrypter(&self) -> &'static RsaesJweDecrypter {
        &DECRYPTER
    }

    fn get_issuer(&self) -> &'static str {
        "aesterisk/daemon"
    }

    async fn on_accept(&self, addr: SocketAddr, tx: Tx) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        self.state.daemon_channel_map.insert(addr, DaemonSocket {
            tx,
            handshake: None,
        });
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());

        Ok(())
    }

    async fn on_disconnect(&self, addr: SocketAddr) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        let uuid = self.state.daemon_channel_map.get(&addr).ok_or("Daemon not found in DaemonChannelMap")?.handshake.as_ref().ok_or("Daemon hasn't authenticated")?.daemon_uuid;
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        self.state.daemon_channel_map.remove(&addr);
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_ID_MAP", file!(), line!());
        self.state.daemon_id_map.remove(&uuid);
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_ID_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_ID_MAP", file!(), line!());
        self.send_event_from_server(&uuid, EventData::NodeStatus(NodeStatusEvent {
            online: false,
            stats: None,
        })).await
    }

    async fn on_decrypt_error(&self, addr: SocketAddr) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        self.state.daemon_channel_map.get(&addr).ok_or("Client not found in channel_map")?.tx.close_channel();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());

        Ok(())
    }

    #[instrument("daemon", skip(self, packet))]
    async fn on_packet(&self, packet: Packet, addr: SocketAddr) -> Result<(), String> {
        match packet.id {
            ID::DSAuth => {
                self.handle_auth(DSAuthPacket::parse(packet).expect("DSAuthPacket should be Some"), addr).await
            },
            ID::DSHandshakeResponse => {
                self.handle_handshake_response(DSHandshakeResponsePacket::parse(packet).expect("DSHandshakeResponsePacket should be Some"), addr).await
            }
            ID::DSEvent => {
                self.handle_event(DSEventPacket::parse(packet).expect("DSEventPacket should be Some"), addr).await
            },
            _ => {
                Err(format!("Should not receive [SW]* packet: {:?}", packet.id))
            },
        }
    }
}
