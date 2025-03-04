use std::{borrow::Borrow, net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use josekit::jwe::alg::rsaes::{RsaesJweDecrypter, RsaesJweEncrypter};
use packet::{daemon_server::{auth::DSAuthPacket, event::DSEventPacket, handshake_response::DSHandshakeResponsePacket}, Packet, ID};
use sqlx::types::Uuid;
use tracing::{info, instrument};

use crate::{config::CONFIG, db, encryption::DECRYPTER, server::Server, state::{DaemonKeyCache, State, Tx}};

/// `DaemonHandshake` is a struct that contains the information required to send a handshake request
/// to the daemon.
pub struct DaemonHandshake {
    pub daemon_uuid: Uuid,
    pub encrypter: RsaesJweEncrypter,
    pub challenge: String,
}

/// `DaemonSocket` is a struct that contains the transmitting end of the `mpsc::unbounded` channel, to
/// send messages to the daemon, as well as an optional `DaemonHandshake` (if the handshake request
/// has been sent).
pub struct DaemonSocket {
    pub tx: Tx,
    pub handshake: Option<DaemonHandshake>,
}

/// `DaemonServer` is a WebSocket server (implemented by the `Server` trait) that listens for daemon
/// connections.
pub struct DaemonServer {
    state: Arc<State>,
}

struct PublicKeyQuery {
    node_public_key: String,
}

impl DaemonServer {
    /// Constructs a new `DaemonServer` with the provided shared state.
    ///
    /// The server instance holds a reference to the application state, which is used to manage daemon connections and related operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    ///
    /// // Assume State implements Default or has a suitable constructor.
    /// let state = Arc::new(State::default());
    /// let server = DaemonServer::new(state);
    /// ```    pub fn new(state: Arc<State>) -> Self {
        Self {
            state
        }
    }

    /// Retrieves the public key for the daemon identified by the given UUID.
        ///
        /// The function first checks an in-memory cache for the public key. If the key is not found,
        /// it asynchronously queries the database, caches the result, and returns a thread-safe
        /// reference to the public key as an `Arc<Vec<u8>>`.
        ///
        /// # Errors
        ///
        /// Returns an error if the daemon does not exist in the database or if the key cannot be retrieved.
        ///
        /// # Examples
        ///
        /// ```
        /// # use std::sync::Arc;
        /// # use uuid::Uuid;
        /// # async fn example(daemon_server: &DaemonServer) {
        /// let daemon_uuid = Uuid::new_v4();
        /// match daemon_server.query_user_public_key(&daemon_uuid).await {
        ///     Ok(key) => println!("Retrieved key with {} bytes", key.len()),
        ///     Err(err) => eprintln!("Error retrieving key: {}", err),
        /// }
        /// # }
        /// ```
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

    /// Processes an authentication packet by parsing the daemon's UUID, retrieving the user's public key,
    /// and sending a handshake request to the daemon.
    ///
    /// The function extracts the UUID from the given authentication packet. If the UUID cannot be parsed,
    /// it returns an error. Otherwise, it asynchronously queries the corresponding public key and sends
    /// a handshake request to the daemon using that key and the provided address.
    ///
    /// # Errors
    ///
    /// Returns an error if the daemon UUID is invalid or if retrieving the public key fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use std::net::SocketAddr;
    /// use uuid::Uuid;
    /// use your_module::{DaemonServer, DSAuthPacket, State};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     // Create an authentication packet with a valid daemon UUID.
    ///     let daemon_uuid = Uuid::new_v4().to_string();
    ///     let auth_packet = DSAuthPacket { daemon_uuid };
    ///
    ///     // Parse a SocketAddr and create a server instance.
    ///     let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///     let state = Arc::new(State::default());
    ///     let server = DaemonServer::new(state);
    ///
    ///     // Process the authentication packet; returns Ok(()) on success.
    ///     server.handle_auth(auth_packet, addr).await.unwrap();
    /// }
    /// ```
    async fn handle_auth(&self, auth_packet: DSAuthPacket, addr: SocketAddr) -> Result<(), String> {
        let uuid = Uuid::parse_str(&auth_packet.daemon_uuid).map_err(|_| "Could not parse UUID")?;
        let key = self.query_user_public_key(&uuid).await?;

        self.state.send_daemon_handshake_request(addr, uuid, key).await
    }

    /// Handles the handshake response by authenticating a daemon using the provided challenge.
    ///
    /// This asynchronous function extracts the challenge from the handshake response packet and delegates daemon
    /// authentication to the server state using the associated socket address. On successful verification, the daemon
    /// is authenticated and a confirmation message is logged.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// // Assume necessary imports for `DaemonServer`, `State`, and `DSHandshakeResponsePacket`
    ///
    /// // Create an instance of the server state and initialize the DaemonServer.
    /// let state = Arc::new(State::new());
    /// let server = DaemonServer::new(state);
    ///
    /// // Construct a sample handshake response packet with a challenge.
    /// let packet = DSHandshakeResponsePacket {
    ///     challenge: "sample_challenge".to_string(),
    ///     // additional fields if necessary...
    /// };
    ///
    /// // Define the daemon's socket address.
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///
    /// // Process the handshake response and assert that authentication is successful.
    /// let result = server.handle_handshake_response(packet, addr).await;
    /// assert!(result.is_ok());
    /// ```
    async fn handle_handshake_response(&self, handshake_reponse_packet: DSHandshakeResponsePacket, addr: SocketAddr) -> Result<(), String> {
        self.state.authenticate_daemon(addr, handshake_reponse_packet.challenge)?;

        info!("Authenticated");

        Ok(())
    }

    /// Asynchronously processes an event packet received from a daemon.
    ///
    /// Extracts the event data from the provided packet and forwards it to the state's event handler.
    ///
    /// # Parameters
    ///
    /// - `event_packet`: A packet containing event data from a daemon.
    /// - `addr`: The socket address of the daemon that sent the event.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the event was successfully forwarded, or an error message otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn example() -> Result<(), String> {
    /// use std::{net::SocketAddr, sync::Arc};
    /// // Assume State, DaemonServer, and DSEventPacket are defined and implemented accordingly.
    /// let state = Arc::new(State::new());
    /// let server = DaemonServer::new(state);
    ///
    /// let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
    /// let event_packet = DSEventPacket {
    ///     data: "sample event".to_string(),
    ///     // Include other necessary fields as required
    /// };
    ///
    /// server.handle_event(event_packet, addr).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn handle_event(&self, event_packet: DSEventPacket, addr: SocketAddr) -> Result<(), String> {
        // debug!("Event: {:#?}", event_packet);

        self.state.send_event_from_daemon(&addr, event_packet.data).await
    }
}

#[async_trait]
impl Server for DaemonServer {
    /// Returns the tracing name for the daemon server.
    ///
    /// This value is used for identifying the server in logs and tracing operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// // Replace `State` with an appropriate implementation or mock as needed.
    /// let state = Arc::new(Default::default());
    /// let server = DaemonServer::new(state);
    /// assert_eq!(server.get_tracing_name(), "daemon");
    /// ```
    fn get_tracing_name(&self) -> &'static str {
        "daemon"
    }

    /// Returns the binding address for the daemon server.
    ///
    /// This method retrieves the socket address from the global configuration that specifies where the daemon
    /// should listen for incoming connections.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `daemon_server` is a properly initialized DaemonServer instance:
    /// let bind_addr = daemon_server.get_bind_addr();
    /// assert!(!bind_addr.is_empty());
    /// println!("Daemon server will bind to {}", bind_addr);
    /// ```
    fn get_bind_addr(&self) -> &'static str {
        &CONFIG.sockets.daemon
    }

    /// Returns a static reference to the JSON Web Encryption decrypter used by the server.
    ///
    /// This decrypter is the globally configured instance for decrypting JWE tokens and is shared across all daemon connections.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// // Assume that `State` and `DaemonServer` are properly imported from the module.
    /// // The `State` struct is expected to implement `Default` or be initialized accordingly.
    /// let state = Arc::new(State::default());
    /// let server = DaemonServer::new(state);
    /// let decrypter = server.get_decrypter();
    ///
    /// // `decrypter` can now be used to decrypt and validate JWE tokens.
    /// ```
    fn get_decrypter(&self) -> &'static RsaesJweDecrypter {
        &DECRYPTER
    }

    /// Returns the issuer identifier for the daemon server.
    ///
    /// This method returns a static string that is used to identify the daemon server in
    /// authentication and logging contexts.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    ///
    /// // Assume that `State` implements `Default` for simplicity.
    /// let state = Arc::new(State::default());
    /// let server = DaemonServer::new(state);
    /// assert_eq!(server.get_issuer(), "aesterisk/daemon");
    /// ```
    fn get_issuer(&self) -> &'static str {
        "aesterisk/daemon"
    }

    /// Accepts a new daemon connection by adding it to the server's state.
    ///
    /// Registers the daemon identified by its socket address along with its transmission channel,
    /// enabling the server to manage its lifecycle.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # async fn example() -> Result<(), String> {
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// // Assume necessary imports for DaemonServer, State, and Tx.
    /// let state = Arc::new(State::new());
    /// let server = DaemonServer::new(state);
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    /// let tx: Tx = /* initialize the transmission channel for the daemon */;
    ///
    /// server.on_accept(addr, tx).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn on_accept(&self, addr: SocketAddr, tx: Tx) -> Result<(), String> {
        self.state.add_daemon(addr, tx);

        Ok(())
    }

    /// Handles daemon disconnection by removing the associated daemon from the server's state.
    ///
    /// This asynchronous method updates the internal state when a daemon disconnects,
    /// ensuring that stale connections are properly cleaned up.
    ///
    /// # Arguments
    ///
    /// * `addr` - The socket address of the daemon that has disconnected.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// use my_crate::{DaemonServer, State}; // Adjust the crate and module paths as needed
    ///
    /// // Initialize the server with a dummy state.
    /// let state = Arc::new(State::new());
    /// let server = DaemonServer::new(state);
    ///
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///
    /// // Remove the daemon from the state on disconnect.
    /// let result = tokio_test::block_on(server.on_disconnect(addr));
    /// assert!(result.is_ok());
    /// ```
    async fn on_disconnect(&self, addr: SocketAddr) -> Result<(), String> {
        self.state.remove_daemon(addr).await
    }

    /// Disconnects the daemon associated with the given socket address after a decryption error.
    ///
    /// This method notifies the server state to remove the daemon connection corresponding to the
    /// address where a decryption error occurred.
    ///
    /// # Arguments
    ///
    /// * `addr` - The socket address of the daemon that encountered the decryption error.
    ///
    /// # Returns
    ///
    /// A `Result` that is `Ok(())` if the disconnection was successful, or an `Err` with an error message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # async fn example() -> Result<(), String> {
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    ///
    /// // Assume `State` and `DaemonServer` are defined and imported accordingly.
    /// let state = Arc::new(State::new());
    /// let server = DaemonServer::new(state);
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///
    /// // Trigger decryption error handling which disconnects the daemon.
    /// server.on_decrypt_error(addr).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn on_decrypt_error(&self, addr: SocketAddr) -> Result<(), String> {
        self.state.disconnect_daemon(addr)
    }

    #[instrument("daemon", skip(self, packet))]
    /// Processes an incoming packet by dispatching it to the appropriate handler based on its identifier.
    ///
    /// This asynchronous function examines the packet's `id` field and routes the packet to one of three handlers:
    /// - Authentication for packets with identifier `DSAuth`.
    /// - Handshake response handling for packets with identifier `DSHandshakeResponse`.
    /// - Event processing for packets with identifier `DSEvent`.
    ///
    /// For any other packet identifier, it returns an error indicating an unexpected packet type.
    ///
    /// # Arguments
    ///
    /// * `packet` - The incoming packet whose `id` determines the processing route.
    /// * `addr` - The socket address from which the packet originated.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the packet is successfully handled; otherwise, returns an `Err` with an error message.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn run() -> Result<(), String> {
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// // Import the required types from the daemon module.
    /// // use your_crate::{DaemonServer, Packet, ID, State};
    ///
    /// // Construct a dummy DSAuth packet. Additional fields are omitted for brevity.
    /// let packet = Packet {
    ///     id: ID::DSAuth,
    ///     // ... other necessary packet fields
    /// };
    ///
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    /// let state = Arc::new(State::default());
    /// let server = DaemonServer::new(state);
    ///
    /// // Process the packet. This will dispatch to the authentication handler.
    /// let result = server.on_packet(packet, addr).await;
    /// assert!(result.is_ok());
    /// # Ok(())
    /// # }
    /// # tokio_test::block_on(run()).unwrap();
    /// ```
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
