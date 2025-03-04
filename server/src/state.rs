use std::{borrow::Borrow, collections::{HashMap, HashSet}, fmt::Write, net::SocketAddr, sync::Arc};

use dashmap::DashMap;
use futures_channel::mpsc;
use openssl::rand::rand_bytes;
use packet::{events::{EventData, EventType, ListenEvent, NodeStatusEvent}, server_daemon::{auth_response::SDAuthResponsePacket, handshake_request::SDHandshakeRequestPacket, listen::SDListenPacket}, server_web::{auth_response::SWAuthResponsePacket, event::SWEventPacket, handshake_request::SWHandshakeRequestPacket}};
use sqlx::types::Uuid;
use tokio_tungstenite::tungstenite::Message;
use tracing::warn;

use crate::{daemon::{DaemonHandshake, DaemonSocket}, encryption, web::{WebHandshake, WebSocket}};

/// `Tx` is a type alias for the transmitting end of an `mpsc::unbounded` channel.
pub type Tx = mpsc::UnboundedSender<Message>;
/// `Rx` is a type alias for the receiving end of an `mpsc::unbounded` channel.
pub type Rx = mpsc::UnboundedReceiver<Message>;

/// `WebChannelMap` is a type alias for a `DashMap` mapping a `SocketAddr` to a `WebSocket`.
pub type WebChannelMap = Arc<DashMap<SocketAddr, WebSocket>>;
/// `DaemonChannelMap` is a type alias for a `DashMap` mapping a user id (`u32`) to a key
/// (`Arc<Vec<u8>>`).
pub type WebKeyCache = Arc<DashMap<u32, Arc<Vec<u8>>>>;

/// `DaemonChannelMap` is a type alias for a `DashMap` mapping a `SocketAddr` to a `DaemonSocket`.
pub type DaemonChannelMap = Arc<DashMap<SocketAddr, DaemonSocket>>;
/// `DaemonKeyCache` is a type alias for a `DashMap` mapping a `Uuid` to a key (`Arc<Vec<u8>>`).
pub type DaemonKeyCache = Arc<DashMap<Uuid, Arc<Vec<u8>>>>;

/// `DaemonListenMap` is a type alias for a `DashMap` mapping a `Uuid` to a `HashMap` of
/// `EventType` to a `HashSet` of `SocketAddr`. Basically, it maps a daemon to a list of events
/// which knows how many clients is currently listening to it.
pub type DaemonListenMap = Arc<DashMap<Uuid, HashMap<EventType, HashSet<SocketAddr>>>>;
/// `WebListenMap` is a type alias for a `DashMap` mapping a `SocketAddr` to a `HashMap` of
/// `EventType` to a `HashSet` of `Uuid`. Basically, it maps a web client to a list of events which
/// knows which daemons to send to.
pub type WebListenMap = Arc<DashMap<SocketAddr, HashMap<EventType, HashSet<Uuid>>>>;
/// `DaemonIDMap` is a type alias for a `DashMap` mapping a `Uuid` to a `SocketAddr`.
pub type DaemonIDMap = Arc<DashMap<Uuid, SocketAddr>>;

/// `State` is a struct containing all data that is required by `daemon` and `web` servers.
pub struct State {
    web_channel_map: WebChannelMap,
    pub web_key_cache: WebKeyCache,

    daemon_channel_map: DaemonChannelMap,
    pub daemon_key_cache: DaemonKeyCache,

    daemon_listen_map: DaemonListenMap,
    web_listen_map: WebListenMap,
    daemon_id_map: DaemonIDMap,
}

impl State {
    /// Creates a new `State` instance.
    pub fn new() -> Self {
        Self {
            web_channel_map: Arc::new(DashMap::new()),
            web_key_cache: Arc::new(DashMap::new()),
            daemon_channel_map: Arc::new(DashMap::new()),
            daemon_key_cache: Arc::new(DashMap::new()),
            daemon_listen_map: Arc::new(DashMap::new()),
            web_listen_map: Arc::new(DashMap::new()),
            daemon_id_map: Arc::new(DashMap::new()),
        }
    }

    /// Sends an event from the server to the web clients listening.
    pub async fn send_event_from_server(&self, uuid: &Uuid, event: EventData) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
        let map: &DaemonListenMap = self.daemon_listen_map.borrow();

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());
        let daemon = map.get(uuid).ok_or("Daemon not found in DaemonListenMap")?;

        let clients = daemon.get(&event.event_type());

        if let Some(clients) = clients {
            for client in clients.iter() {
                #[cfg(feature = "lock_debug")]
                debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
                let map: &WebChannelMap = self.web_channel_map.borrow();

                #[cfg(feature = "lock_debug")]
                debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());
                let socket = map.get(client).ok_or("Disconnected client still in WebChannelMap")?;

                socket.tx.unbounded_send(
                    Message::Text(
                        encryption::encrypt_packet(
                            SWEventPacket {
                                event: event.clone(),
                                daemon: *uuid,
                            }.to_packet()?,
                            &socket.handshake.as_ref().ok_or("Client hasn't requested authentication")?.encrypter
                        )?
                    )
                ).map_err(|_| "Could not send packet to client")?;

                #[cfg(feature = "lock_debug")]
                debug!("[{}:{}] dropped WEB_CHANNEL_MAP", file!(), line!());
            }
        }

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_LISTEN_MAP", file!(), line!());

        Ok(())
    }

    /// Sends an event from the daemon to the server.
    pub async fn send_event_from_daemon(&self, addr: &SocketAddr, event: EventData) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        let uuid = self.daemon_channel_map.get(addr).ok_or("Daemon not found in DaemonChannelMap")?.handshake.as_ref().ok_or("Client hasn't requested authentication")?.daemon_uuid;

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());

        self.send_event_from_server(&uuid, event).await
    }

    /// Sends a handshake request to a daemon.
    pub async fn send_daemon_handshake_request(&self, addr: SocketAddr, uuid: Uuid, key: Arc<Vec<u8>>) -> Result<(), String> {
        let mut challenge_bytes = [0; 256];
        rand_bytes(&mut challenge_bytes).map_err(|_| "Could not generate challenge")?;

        let challenge = challenge_bytes.iter().try_fold::<_, _, Result<String, String>>(String::new(), |mut s, byte| {
            write!(s, "{:02X}", byte).map_err(|_| "could not write byte")?;
            Ok(s)
        })?;

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        let clients: &DaemonChannelMap = self.daemon_channel_map.borrow();

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
                encryption::encrypt_packet(
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

    /// Authenticates a daemon with the given challenge.
    pub fn authenticate_daemon(&self, addr: SocketAddr, challenge: String) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        let clients: &DaemonChannelMap = self.daemon_channel_map.borrow();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
        let client = clients.get(&addr).ok_or("Client not found in channel_map")?;

        if challenge != client.handshake.as_ref().ok_or("Client hasn't requested authentication")?.challenge {
            warn!("Failed authentication");
            client.tx.close_channel();
            return Err("Challenge does not match".to_string());
        }

        let uuid = client.handshake.as_ref().ok_or("Client hasn't requested authentication")?.daemon_uuid;
        let encrypter = &client.handshake.as_ref().ok_or("Client hasn't requested authentication")?.encrypter;

        client.tx.unbounded_send(
            Message::text(
                encryption::encrypt_packet(
                    SDAuthResponsePacket {
                        success: true,
                    }.to_packet()?,
                    encrypter,
                )?
            )
        ).map_err(|_| "Failed to send packet")?;

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
        let daemon_listen_map: &DaemonListenMap = self.daemon_listen_map.borrow();

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());
        if let Some(listen_map) = daemon_listen_map.get(&uuid) {
            let events = listen_map.keys().copied().collect::<Vec<_>>();

            client.tx.unbounded_send(
                Message::Text(
                    encryption::encrypt_packet(
                        SDListenPacket {
                            events
                        }.to_packet()?,
                        encrypter
                    )?
                )
            ).map_err(|_| "Failed to send packet")?;
        }

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_ID_MAP", file!(), line!());
        self.daemon_id_map.insert(uuid, addr);

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

    /// Adds a daemon to the server.
    pub fn add_daemon(&self, addr: SocketAddr, tx: Tx) {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        self.daemon_channel_map.insert(addr, DaemonSocket {
            tx,
            handshake: None,
        });

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());
    }

    /// Removes a daemon from the server. Should only be used in the `on_disconnect` method, see
    /// `disconnect_daemon` for a more general use case.
    pub async fn remove_daemon(&self, addr: SocketAddr) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        let uuid = self.daemon_channel_map.get(&addr).ok_or("Daemon not found in DaemonChannelMap")?.handshake.as_ref().ok_or("Daemon hasn't authenticated")?.daemon_uuid;
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        self.daemon_channel_map.remove(&addr);
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_ID_MAP", file!(), line!());
        self.daemon_id_map.remove(&uuid);
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_ID_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_ID_MAP", file!(), line!());

        self.send_event_from_server(&uuid, EventData::NodeStatus(NodeStatusEvent {
            online: false,
            stats: None,
        })).await
    }

    /// Disconnects a daemon from the server.
    pub fn disconnect_daemon(&self, addr: SocketAddr) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        self.daemon_channel_map.get(&addr).ok_or("Client not found in channel_map")?.tx.close_channel();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());

        Ok(())
    }

    /// Called when a daemon connects to the server to immediately send it all events that has been
    /// listened to.
    pub async fn update_listens_for_daemon(&self, addr: &SocketAddr, uuid: &Uuid) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        let daemon_channel_map: &DaemonChannelMap = self.daemon_channel_map.borrow();

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());
        let socket = daemon_channel_map.get(addr).ok_or("Daemon not found in DaemonChannelMap")?;

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
        let daemon_listen_map: &DaemonListenMap = self.daemon_listen_map.borrow();

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());
        let events = daemon_listen_map.get(uuid).ok_or("Daemon not found in DaemonListenMap")?.keys().copied().collect::<Vec<_>>();

        socket.tx.unbounded_send(
            Message::Text(
                encryption::encrypt_packet(
                    SDListenPacket {
                        events
                    }.to_packet()?,
                    &socket.handshake.as_ref().ok_or("Daemon hasn't requested authentication!")?.encrypter
                )?
            )
        ).map_err(|_| "Failed to send packet")?;

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_LISTEN_MAP", file!(), line!());
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());

        Ok(())
    }

    /// Sends a handshake request to a web client.
    pub fn send_web_handshake_request(&self, addr: &SocketAddr, user_id: u32, key: Arc<Vec<u8>>) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
        let clients: &WebChannelMap = self.web_channel_map.borrow();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());
        let mut client = clients.get_mut(addr).ok_or("Client not found in channel_map")?;

        let mut challenge_bytes = [0; 256];
        rand_bytes(&mut challenge_bytes).map_err(|_| "Could not generate challenge")?;
        let challenge = challenge_bytes.iter().try_fold::<_, _, Result<String, String>>(String::new(), |mut s, byte| {
            write!(s, "{:02X}", byte).map_err(|_| "could not write byte")?;
            Ok(s)
        })?;

        client.handshake = Some(WebHandshake {
            user_id,
            encrypter: josekit::jwe::RSA_OAEP.encrypter_from_pem(key.as_ref()).map_err(|_| "key should be valid")?,
            challenge: challenge.clone(),
        });

        client.tx.unbounded_send(
            Message::text(
                encryption::encrypt_packet(
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

    /// Authenticates a web client with the given challenge.
    pub fn authenticate_web(&self, addr: SocketAddr, challenge: String) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
        let clients: &WebChannelMap = self.web_channel_map.borrow();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());
        let client = clients.get_mut(&addr).ok_or("Client not found in channel_map")?;

        if challenge != client.handshake.as_ref().ok_or("Client hasn't requested authentication")?.challenge {
            warn!("Failed authentication");
            client.tx.close_channel();
            return Err("Challenge does not match".to_string());
        }

        client.tx.unbounded_send(
            Message::text(
                encryption::encrypt_packet(
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

    /// Forwards a listen event to all daemons required from a web client.
    pub async fn send_listen(&self, addr: SocketAddr, events: Vec<ListenEvent>) -> Result<(), String> {
        let mut update_daemons = HashSet::new();
        let mut offline_daemons = HashSet::new();

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_ID_MAP", file!(), line!());
        let daemon_id_map: &DaemonIDMap = self.daemon_id_map.borrow();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_ID_MAP", file!(), line!());

        {
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] awaiting WEB_LISTEN_MAP", file!(), line!());
            let web_listen_map: &WebListenMap = self.web_listen_map.borrow();
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] got WEB_LISTEN_MAP", file!(), line!());

            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
            let daemon_listen_map: &DaemonListenMap = self.daemon_listen_map.borrow();
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());

            for event in events.into_iter() {
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

    /// Adds a web client to the server.
    pub fn add_web(&self, addr: SocketAddr, tx: Tx) {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());

        self.web_channel_map.insert(addr, WebSocket {
            tx,
            handshake: None,
        });

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped WEB_CHANNEL_MAP", file!(), line!());
    }

    /// Removes a web client from the server. Should only be used in the `on_disconnect` method,
    /// see `disconnect_web` for a more general use case.
    pub async fn remove_web(&self, addr: SocketAddr) -> Result<(), String> {
        let mut update_daemons = HashSet::new();

        {
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] awaiting WEB_LISTEN_MAP", file!(), line!());
            let web_listen_map: &WebListenMap = self.web_listen_map.borrow();
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] got WEB_LISTEN_MAP", file!(), line!());

            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] awaiting DAEMON_LISTEN_MAP", file!(), line!());
            let daemon_listen_map: &DaemonListenMap = self.daemon_listen_map.borrow();
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] got DAEMON_LISTEN_MAP", file!(), line!());

            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
            let web_channel_map: &WebChannelMap = self.web_channel_map.borrow();
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
            if let Some(daemon_addr) = self.daemon_id_map.get(&daemon) {
                self.update_listens_for_daemon(&daemon_addr, &daemon).await?;
            }
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] got DAEMON_ID_MAP", file!(), line!());
            #[cfg(feature = "lock_debug")]
            debug!("[{}:{}] dropped DAEMON_ID_MAP", file!(), line!());
        }

        Ok(())
    }

    /// Disconnects a web client from the server.
    pub fn disconnect_web(&self, addr: SocketAddr) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
        self.web_channel_map.get(&addr).ok_or("Client not found in channel_map")?.tx.close_channel();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped WEB_CHANNEL_MAP", file!(), line!());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{pin::Pin, str::FromStr};

    use futures_util::StreamExt;
    use josekit::jwk;
    use mpsc::unbounded;
    use packet::ID;

    use super::*;

    #[tokio::test]
    async fn encryption_decryption() {
        let state = Arc::new(State::new());

        let web_addr_1 = SocketAddr::from(([127, 0, 0, 1], 30001));
        let (web_tx_1, mut web_rx_1) = unbounded();

        let web_keys_1 = jwk::alg::rsa::RsaKeyPair::generate(2048).expect("could not create keys");
        let web_public_1 = Arc::new(web_keys_1.to_pem_public_key());

        let web_private_1 = Arc::new(web_keys_1.to_pem_private_key());
        let decrypter = josekit::jwe::RSA_OAEP.decrypter_from_pem(web_private_1.as_ref()).expect("could not create decrypter");

        state.add_web(web_addr_1, web_tx_1);
        state.send_web_handshake_request(&web_addr_1, 1, web_public_1).expect("could not send web handshake request");

        let handshake_request = web_rx_1.next().await.expect("could not get message");
        let message = handshake_request.into_text().expect("message is not text");

        let packet = encryption::decrypt_packet(&message, &decrypter, "aesterisk/server", None::<fn() -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>>).await.expect("could not decrypt packet");

        assert_eq!(packet.id, ID::SWHandshakeRequest);
    }

    #[tokio::test]
    async fn web_authentication() {
        let state = Arc::new(State::new());

        let web_addr_1 = SocketAddr::from(([127, 0, 0, 1], 30001));
        let (web_tx_1, mut web_rx_1) = unbounded();

        let web_keys_1 = jwk::alg::rsa::RsaKeyPair::generate(2048).expect("could not create keys");
        let web_public_1 = Arc::new(web_keys_1.to_pem_public_key());

        let web_private_1 = Arc::new(web_keys_1.to_pem_private_key());
        let decrypter = josekit::jwe::RSA_OAEP.decrypter_from_pem(web_private_1.as_ref()).expect("could not create decrypter");

        let web_user_id_1 = 1234;

        state.add_web(web_addr_1, web_tx_1);
        state.send_web_handshake_request(&web_addr_1, web_user_id_1, web_public_1).expect("could not send web handshake request");

        let handshake_request = web_rx_1.next().await.expect("could not get message");
        let message = handshake_request.into_text().expect("message is not text");

        let packet = encryption::decrypt_packet(&message, &decrypter, "aesterisk/server", None::<fn() -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>>).await.expect("could not decrypt packet");

        assert_eq!(packet.id, ID::SWHandshakeRequest);

        let handshake_request = SWHandshakeRequestPacket::parse(packet).expect("could not parse packet");

        state.authenticate_web(web_addr_1, handshake_request.challenge).expect("could not authenticate");

        let client = state.web_channel_map.get(&web_addr_1);
        assert!(client.is_some());
        assert!(client.as_ref().unwrap().handshake.is_some());
        assert!(client.unwrap().handshake.as_ref().unwrap().user_id == web_user_id_1);
    }

    #[tokio::test]
    async fn daemon_authentication() {
        let state = Arc::new(State::new());

        let daemon_addr_1 = SocketAddr::from(([127, 0, 0, 1], 30001));
        let (daemon_tx_1, mut daemon_rx_1) = unbounded();

        let daemon_keys_1 = jwk::alg::rsa::RsaKeyPair::generate(2048).expect("could not create keys");
        let daemon_public_1 = Arc::new(daemon_keys_1.to_pem_public_key());

        let daemon_private_1 = Arc::new(daemon_keys_1.to_pem_private_key());
        let decrypter = josekit::jwe::RSA_OAEP.decrypter_from_pem(daemon_private_1.as_ref()).expect("could not create decrypter");

        let daemon_uuid_1 = Uuid::from_str("DAE11071-0000-4000-0000-000000000000").expect("could not create uuid");

        state.add_daemon(daemon_addr_1, daemon_tx_1);
        state.send_daemon_handshake_request(daemon_addr_1, daemon_uuid_1, daemon_public_1).await.expect("could not send daemon handshake request");

        let handshake_request = daemon_rx_1.next().await.expect("could not get message");
        let message = handshake_request.into_text().expect("message is not text");

        let packet = encryption::decrypt_packet(&message, &decrypter, "aesterisk/server", None::<fn() -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>>).await.expect("could not decrypt packet");

        assert_eq!(packet.id, ID::SDHandshakeRequest);

        let handshake_request = SDHandshakeRequestPacket::parse(packet).expect("could not parse packet");

        state.authenticate_daemon(daemon_addr_1, handshake_request.challenge).expect("could not authenticate");

        let client = state.daemon_channel_map.get(&daemon_addr_1);
        assert!(client.is_some());
        assert!(client.as_ref().unwrap().handshake.is_some());
        assert!(client.unwrap().handshake.as_ref().unwrap().daemon_uuid == daemon_uuid_1);
    }
}
