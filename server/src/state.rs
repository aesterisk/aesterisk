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
    /// Creates a new `State` instance with all internal mappings initialized.
    ///
    /// This function sets up empty concurrent maps for managing communication channels,
    /// encryption keys, and event subscriptions between web clients and daemon servers.
    /// Use it to obtain a clean state for asynchronous communication management.
    ///
    /// # Examples
    ///
    /// ```
    /// use server::state::State;
    ///
    /// let state = State::new();
    /// // The state is now ready for use with empty mappings.
    /// ```    pub fn new() -> Self {
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

    /// Sends an event from the server to all web clients subscribed to the event's type.
    ///
    /// This method retrieves the set of web clients that are listening for the event associated with the given daemon UUID,
    /// encrypts the event packet using each client's handshake encryption key, and sends it over their communication channel.
    /// It returns an error if the daemon is not found in the listening map, if any client information (such as its channel or
    /// handshake details) is missing, or if sending the packet fails.
    ///
    /// # Errors
    ///
    /// Returns an `Err(String)` if:
    /// - The daemon associated with the provided UUID is not found in the daemon listen map.
    /// - A subscribed web client's channel is missing from the web channel map.
    /// - The client's handshake information is unavailable.
    /// - Sending the encrypted message to a client fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use uuid::Uuid;
    /// // Assume `State` and `EventData` are imported from the relevant module.
    ///
    /// # use server::state::State;
    /// # use server::event::EventData;
    /// # async fn run_example() -> Result<(), String> {
    /// // Initialize a new state instance.
    /// let state = State::new();
    ///
    /// // Create a dummy daemon UUID.
    /// let daemon_id = Uuid::new_v4();
    ///
    /// // Construct a dummy event. Replace with the actual event initializer as needed.
    /// let event = EventData::default();
    ///
    /// // Send the event from the server to subscribed web clients.
    /// state.send_event_from_server(&daemon_id, event).await?;
    /// # Ok(())
    /// # }
    /// # tokio_test::block_on(run_example()).unwrap();
    /// ```    pub async fn send_event_from_server(&self, uuid: &Uuid, event: EventData) -> Result<(), String> {
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

    /// Sends an event from a daemon to the server.
    ///
    /// This asynchronous function retrieves the daemon's unique identifier from the internal
    /// channel map using its network address. If the daemon is registered and has initiated
    /// a handshake for authentication, the function forwards the specified event to the server's
    /// event handling mechanism, which dispatches it to the appropriate web clients.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The daemon address is not found in the channel map.
    /// - The daemon's handshake data is missing, indicating that authentication was not requested.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// use std::net::SocketAddr;
    /// use your_crate::{State, EventData};
    ///
    /// // Initialize the server state.
    /// let state = State::new();
    ///
    /// // Define the daemon's address (adjust as needed).
    /// let daemon_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///
    /// // Create an event (initialize accordingly).
    /// let event = EventData::new("example_event", vec![]);
    ///
    /// // Send the event from the daemon to the server.
    /// if let Err(e) = state.send_event_from_daemon(&daemon_addr, event).await {
    ///     eprintln!("Failed to send event: {}", e);
    /// }
    /// # }
    /// ```    pub async fn send_event_from_daemon(&self, addr: &SocketAddr, event: EventData) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting DAEMON_CHANNEL_MAP", file!(), line!());
        let uuid = self.daemon_channel_map.get(addr).ok_or("Daemon not found in DaemonChannelMap")?.handshake.as_ref().ok_or("Client hasn't requested authentication")?.daemon_uuid;

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got DAEMON_CHANNEL_MAP", file!(), line!());

        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] dropped DAEMON_CHANNEL_MAP", file!(), line!());

        self.send_event_from_server(&uuid, event).await
    }

    /// Initiates a handshake with a daemon by generating a cryptographic challenge and sending an encrypted handshake request.
    ///
    /// This asynchronous function creates a 256-byte random challenge (formatted as an uppercase hexadecimal string) and assigns a handshake
    /// session to the daemon identified by the given network address. It constructs an encrypter using RSA OAEP with the provided PEM key,
    /// encrypts a handshake request packet containing the challenge, and transmits it over the daemon's communication channel.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The cryptographic challenge cannot be generated.
    /// - The daemon is not found in the channel map.
    /// - The provided key is invalid.
    /// - The daemon has not initiated an authentication request.
    /// - Sending the encrypted handshake packet fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use uuid::Uuid;
    /// use tokio::sync::mpsc;
    /// // Replace `your_crate` with the actual crate name where `State` is defined.
    /// use your_crate::state::State;
    ///
    /// #[tokio::test]
    /// async fn test_send_daemon_handshake_request() {
    ///     // Setup a new state and a dummy daemon entry.
    ///     let state = State::new();
    ///     let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///     let uuid = Uuid::new_v4();
    ///
    ///     // Establish a dummy communication channel for the daemon.
    ///     let (tx, _rx) = mpsc::unbounded_channel();
    ///     state.add_daemon(addr, tx);
    ///
    ///     // Provide a valid PEM-encoded public key for testing purposes.
    ///     let pem_key = br#"-----BEGIN PUBLIC KEY-----
    /// MFwwDQYJKoZIhvcNAQEBBQADSwAwSAJBAMZ2kQN3zW5GYlBd7dP76gID+YxdiPjp
    /// K5fjOEBp5QqOCB7LMMbG4F7QKlBtn4tF9Kd8QFZysjZwTsN1N+2kR+YCAwEAAQ==
    /// -----END PUBLIC KEY-----"#.to_vec();
    ///     let key = Arc::new(pem_key);
    ///
    ///     let result = state.send_daemon_handshake_request(addr, uuid, key).await;
    ///     assert!(result.is_ok());
    /// }
    /// ```    pub async fn send_daemon_handshake_request(&self, addr: SocketAddr, uuid: Uuid, key: Arc<Vec<u8>>) -> Result<(), String> {
        let mut challenge_bytes = [0; 256];
        rand_bytes(&mut challenge_bytes).map_err(|_| "Could not generate challenge")?;

        let challenge = challenge_bytes.iter().fold(String::new(), |mut s, byte| {
            write!(s, "{:02X}", byte).expect("could not write byte");
            s
        });

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

    /// Authenticates a daemon by validating its handshake challenge.
    /// 
    /// The function retrieves the daemon's handshake information from the channel map using its socket
    /// address. It then checks if the provided challenge matches the one stored during the handshake.
    /// On a successful match, an encrypted authentication response is sent back to the daemon. If there
    /// are any pending event subscriptions for the daemon, those are forwarded as an encrypted listen
    /// packet. Finally, the daemon is registered in the ID map for further communication.
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - The client corresponding to the given address is not found,
    /// - No handshake was initiated for the client,
    /// - The provided challenge does not match the expected value, or
    /// - Sending a response packet fails.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use std::net::SocketAddr;
    /// // Assume that State is properly set up and a daemon handshake has been initiated.
    /// let state = State::new();
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    /// let challenge = "expected_challenge".to_string();
    /// 
    /// // Attempt to authenticate the daemon. On success, Ok(()) is returned.
    /// state.authenticate_daemon(addr, challenge)
    ///     .expect("Daemon authentication failed");
    /// ```    pub fn authenticate_daemon(&self, addr: SocketAddr, challenge: String) -> Result<(), String> {
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

    /// Registers a daemon's communication channel with the server.
    ///
    /// This method adds a new daemon socket to the server's daemon channel map by associating
    /// the daemon's network address with its message transmitter. The associated daemon socket
    /// is initialized without any handshake information.
    ///
    /// # Arguments
    ///
    /// * `addr` - The network address of the daemon.
    /// * `tx`   - The channel transmitter used to send messages to the daemon.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// use tokio::sync::mpsc::unbounded_channel;
    ///
    /// // Create a new instance of the state.
    /// let state = State::new();
    ///
    /// // Define a daemon address.
    /// let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    ///
    /// // Create an unbounded channel for daemon communication.
    /// let (tx, _rx) = unbounded_channel();
    ///
    /// // Register the daemon with the server.
    /// state.add_daemon(addr, tx);
    /// ```    pub fn add_daemon(&self, addr: SocketAddr, tx: Tx) {
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
    /// Removes a daemon connection from the server state and notifies clients that the daemon is offline.
    ///
    /// This asynchronous function removes the daemon identified by its socket address from the server's connection maps.
    /// It first retrieves the daemon’s UUID by verifying that the daemon has authenticated (via its handshake state),
    /// then removes the connection from both the daemon channel map and the daemon ID map. Finally, it sends an event
    /// to web clients to update the node status to offline.
    ///
    /// # Errors
    ///
    /// Returns an error if the daemon is not found in the connection map or if it has not successfully authenticated.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use tokio;
    /// // Replace `your_crate` with the appropriate module path.
    /// # use your_crate::State;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let state = State::new();
    ///     let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///
    ///     // Removing a daemon that isn't registered yields an error.
    ///     let result = state.remove_daemon(addr).await;
    ///     assert!(result.is_err());
    /// }
    /// ```    pub async fn remove_daemon(&self, addr: SocketAddr) -> Result<(), String> {
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

    /// Disconnects a daemon from the server by closing its communication channel.
    ///
    /// The function attempts to locate the daemon using the provided address in the server's channel map and then closes its associated transmit channel.
    /// If the daemon is not found, an error is returned.
    ///
    /// # Errors
    ///
    /// Returns an `Err` with a descriptive message if the daemon associated with the specified address is not found.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// // Assume `State` is properly initialized and `create_dummy_tx()` is a helper
    /// // that creates a valid Tx instance for testing purposes.
    /// let state = State::new();
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///
    /// // Add a daemon connection with a dummy transmission channel.
    /// state.add_daemon(addr, create_dummy_tx());
    ///
    /// // Disconnecting the daemon should succeed.
    /// assert!(state.disconnect_daemon(addr).is_ok());
    /// ```    pub fn disconnect_daemon(&self, addr: SocketAddr) -> Result<(), String> {
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
    /// Updates a daemon's event subscriptions by sending the current listen events as an encrypted packet.
    ///
    /// This asynchronous method retrieves the list of event types a daemon (identified by its UUID) is subscribed to
    /// from the internal listen mapping. It then fetches the daemon's communication channel associated with the provided
    /// socket address, constructs a packet containing these events, encrypts it using the daemon's handshake encryption,
    /// and sends it over the channel.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The daemon is not found in the daemon channel or listen maps.
    /// - The daemon has not completed the required handshake for encryption.
    /// - Sending the encrypted packet fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use uuid::Uuid;
    /// use tokio;
    ///
    /// // Assume a State instance with a configured daemon channel and listen map.
    /// #[tokio::main]
    /// async fn main() {
    ///     let state = State::new();
    ///     let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///     let daemon_uuid = Uuid::new_v4(); // Example daemon UUID
    ///
    ///     // The daemon should be added to the state with a valid channel and handshake state prior to this call.
    ///     match state.update_listens_for_daemon(&addr, &daemon_uuid).await {
    ///         Ok(()) => println!("Daemon listen events updated successfully."),
    ///         Err(e) => eprintln!("Failed to update daemon listens: {}", e),
    ///     }
    /// }
    /// ```    pub async fn update_listens_for_daemon(&self, addr: &SocketAddr, uuid: &Uuid) -> Result<(), String> {
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

    /// Sends a handshake request to a web client by generating a cryptographic challenge and transmitting an encrypted handshake packet.
    ///
    /// This method creates a random challenge, sets the handshake state for the targeted web client using the provided user identifier,
    /// and encrypts the handshake request using RSA OAEP with the given PEM-encoded key. The handshake packet is then sent to the client,
    /// which must respond with the challenge to complete authentication.
    ///
    /// # Arguments
    ///
    /// * `addr` - The socket address of the web client to which the handshake request is sent.
    /// * `user_id` - The identifier for the web client initiating the handshake.
    /// * `key` - An RSA public key (in PEM format) used to construct the encrypter for securing the handshake packet.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The specified client is not found in the channel map.
    /// - A cryptographic challenge could not be generated.
    /// - The provided key is invalid for creating an encrypter.
    /// - The handshake packet fails to send.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    ///
    /// // Assume `state` is an initialized instance of State with a registered web client at the given address.
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    /// let user_id = 42;
    /// let key_data = b"-----BEGIN PUBLIC KEY-----\n...\n-----END PUBLIC KEY-----";
    /// let key = Arc::new(key_data.to_vec());
    ///
    /// let result = state.send_web_handshake_request(&addr, user_id, key);
    /// assert!(result.is_ok());
    /// ```    pub fn send_web_handshake_request(&self, addr: &SocketAddr, user_id: u32, key: Arc<Vec<u8>>) -> Result<(), String> {
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] awaiting WEB_CHANNEL_MAP", file!(), line!());
        let clients: &WebChannelMap = self.web_channel_map.borrow();
        #[cfg(feature = "lock_debug")]
        debug!("[{}:{}] got WEB_CHANNEL_MAP", file!(), line!());
        let mut client = clients.get_mut(addr).ok_or("Client not found in channel_map")?;

        let mut challenge_bytes = [0; 256];
        rand_bytes(&mut challenge_bytes).map_err(|_| "Could not generate challenge")?;
        let challenge = challenge_bytes.iter().fold(String::new(), |mut s, byte| {
            write!(s, "{:02X}", byte).expect("could not write byte");
            s
        });

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

    /// Authenticates a web client by verifying a provided challenge against its handshake request.
    /// 
    /// This method retrieves the client associated with the provided address and checks that the given
    /// challenge string matches the one stored in the client's handshake state. If the challenges match,
    /// it encrypts and sends an authentication response packet to the client using the handshake encrypter.
    /// In case of a mismatch or any failure (e.g. client not found, missing handshake, or failure to send
    /// the packet), the client's channel is closed and an error is returned.
    /// 
    /// # Errors
    /// 
    /// Returns an error if:
    /// - The client is not found in the channel map.
    /// - The client has not initiated an authentication handshake.
    /// - The provided challenge does not match the expected challenge.
    /// - There is a failure in sending the encrypted authentication response.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use std::net::SocketAddr;
    /// use std::str::FromStr;
    /// 
    /// // Assume that state is an instance of State properly initialized and a client has requested handshake
    /// // so that its handshake field contains the expected challenge and encrypter.
    /// let state = State::new();
    /// let addr = SocketAddr::from_str("127.0.0.1:8080").unwrap();
    /// 
    /// // In a typical setup, ensure the client at addr has an active handshake with challenge "expected_challenge".
    /// // Authenticate the client with the matching challenge.
    /// let result = state.authenticate_web(addr, "expected_challenge".to_string());
    /// assert!(result.is_ok());
    /// ```
    /// 
    /// Note: This method assumes that the handshake request has been received and stored for the client.    pub fn authenticate_web(&self, addr: SocketAddr, challenge: String) -> Result<(), String> {
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

    /// Forwards listen events from a web client to the appropriate daemons and updates subscription mappings.
    /// 
    /// This asynchronous function processes a list of listen events submitted by a web client, identified by its socket address.
    /// For each event, it updates both the daemon and web listen maps: associating the client's address with the event on the daemon side,
    /// and linking the event to the specified daemons on the web side. If a NodeStatus event is received and a daemon is not registered,
    /// the daemon is marked offline and notified accordingly. Finally, for each daemon whose subscriptions were updated and is online,
    /// the function refreshes its listen configuration.
    /// 
    /// Returns `Ok(())` on successful processing or an error message as a `String` if any operation fails.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use std::net::SocketAddr;
    /// use uuid::Uuid;
    /// use your_crate::{State, ListenEvent, EventType, NodeStatusEvent, EventData}; // adjust imports as needed
    /// use tokio;
    /// 
    /// #[tokio::test]
    /// async fn test_send_listen() {
    ///     // Initialize the server state.
    ///     let state = State::new();
    ///     
    ///     // Define a web client's socket address.
    ///     let client_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///     
    ///     // Create a listen event for including a NodeStatus update from a daemon.
    ///     let daemon_id = Uuid::new_v4();
    ///     let listen_event = ListenEvent {
    ///         event: EventType::NodeStatus,
    ///         daemons: vec![daemon_id],
    ///     };
    ///     
    ///     // Forward the listen event.
    ///     let result = state.send_listen(client_addr, vec![listen_event]).await;
    ///     assert!(result.is_ok());
    /// }
    /// ```    pub async fn send_listen(&self, addr: SocketAddr, events: Vec<ListenEvent>) -> Result<(), String> {
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

    /// Registers a new web client with the server.
    ///
    /// Inserts a new web client connection into the server's internal channel map, associating the client's network address with a WebSocket instance configured with the provided transmitter and an unset handshake state.
    ///
    /// # Arguments
    ///
    /// * `addr` - The network address of the web client.
    /// * `tx` - The channel transmitter used to send messages to the web client.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use server::state::State;
    /// use tokio::sync::mpsc::unbounded_channel;
    ///
    /// let state = State::new();
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    /// let (tx, _rx) = unbounded_channel();
    /// state.add_web(addr, tx);
    /// ```    pub fn add_web(&self, addr: SocketAddr, tx: Tx) {
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
    /// Removes a web client's connection from the server state and updates associated daemon event subscriptions.
    ///
    /// This asynchronous function removes the specified web client's channel and event subscriptions from
    /// the internal maps. It also notifies all daemons associated with the web client's events to update
    /// their own subscriptions accordingly.
    ///
    /// # Arguments
    ///
    /// * `addr` - The socket address of the web client to remove.
    ///
    /// # Errors
    ///
    /// Returns an error if a required daemon entry is missing in the daemon listen map or if updating
    /// a daemon's subscriptions fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use server::state::State;
    /// use std::net::SocketAddr;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let state = State::new();
    ///     let web_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///
    ///     // Remove the web client and update subscriptions for associated daemons.
    ///     match state.remove_web(web_addr).await {
    ///         Ok(()) => println!("Web client removed successfully."),
    ///         Err(err) => eprintln!("Failed to remove web client: {}", err),
    ///     }
    /// }
    /// ```    pub async fn remove_web(&self, addr: SocketAddr) -> Result<(), String> {
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

    /// Disconnects a web client from the server by closing its transmission channel.
    ///
    /// This function looks up the web client associated with the provided socket address and,
    /// if found, closes the client's channel. It returns an error if no client is found for the given address.
    ///
    /// # Errors
    ///
    /// Returns an error if the client is not found in the channel map.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// // Assume `state` is an instance of State.
    /// let web_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///
    /// match state.disconnect_web(web_addr) {
    ///     Ok(()) => println!("Web client disconnected successfully."),
    ///     Err(e) => eprintln!("Disconnection failed: {}", e),
    /// }
    /// ```    pub fn disconnect_web(&self, addr: SocketAddr) -> Result<(), String> {
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
    /// Simulates a complete daemon authentication handshake.
    ///
    /// This asynchronous function demonstrates the daemon handshake and authentication process.
    /// It creates a new state, generates RSA key pairs, and initiates a handshake by sending a request
    /// to a simulated daemon. After decrypting and parsing the handshake response, it authenticates the daemon,
    /// and then asserts that the daemon has been correctly registered with the expected handshake details.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use std::net::SocketAddr;
    /// # use uuid::Uuid;
    /// # use futures::Future;
    /// # use some_crate::state::State; // adjust the import path as needed
    /// # #[tokio::main]
    /// # async fn main() {
    ///     daemon_authentication().await;
    /// # }
    /// ```
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
