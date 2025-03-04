use std::{borrow::Borrow, net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use josekit::jwe::alg::rsaes::RsaesJweEncrypter;
use packet::{web_server::{auth::WSAuthPacket, handshake_response::WSHandshakeResponsePacket, listen::WSListenPacket}, Packet, ID};
use tracing::{info, instrument};

use crate::{config::CONFIG, db, encryption::DECRYPTER, server::Server, state::{State, Tx, WebKeyCache}};

/// WebServer is a WebSocket server (implemented by the `Server` trait) that listens for web
/// (frontend) connections.
pub struct WebServer {
    state: Arc<State>,
}

/// WebHandshake is a struct that contains the information required to send a handshake request to
/// the web client.
pub struct WebHandshake {
    #[allow(dead_code)] // TODO: this should be used to authenticate which user can access which
                        //       daemons
    pub user_id: u32,
    pub encrypter: RsaesJweEncrypter,
    pub challenge: String,
}

/// WebSocket is a struct that contains the transmitting end of the `mpsc::unbounded` channel, to
/// send messages to the web client, as well as an optional `WebHandshake` (if the handshake
/// request has been sent).
pub struct WebSocket {
    pub tx: Tx,
    pub handshake: Option<WebHandshake>,
}

struct PublicKeyQuery {
    user_public_key: String,
}

impl WebServer {
    /// Constructs a new `WebServer` instance using the provided application state.
    ///
    /// The application state is shared across the server via an atomic reference count, ensuring safe concurrent access.
    /// This new instance holds the state for managing WebSocket connections and related operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    ///
    /// // Assume `State` and `WebServer` are in scope and `State::default` is available.
    /// let state = Arc::new(State::default());
    /// let server = WebServer::new(state);
    /// ```    pub fn new(state: Arc<State>) -> Self {
        Self {
            state
        }
    }

    /// Retrieves the public key for the specified user ID.
    ///
    /// This asynchronous function first checks an in-memory cache for the user's public key.
    /// If the key is not found, it queries the database, caches the result, and then returns it.
    /// An error is returned if the user does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use your_module::{State, WebServer}; // Replace with actual module paths.
    /// # async fn run_example() -> Result<(), String> {
    /// let state = Arc::new(State::default());
    /// let server = WebServer::new(state);
    /// let user_id = 42;
    /// let public_key = server.query_user_public_key(user_id).await?;
    /// // `public_key` is an Arc<Vec<u8>> containing the user's public key bytes.
    /// # Ok(())
    /// # }
    /// ```
    async fn query_user_public_key(&self, user_id: u32) -> Result<Arc<Vec<u8>>, String> {
        {
            let cache: &WebKeyCache = self.state.web_key_cache.borrow();
            if let Some(v) = cache.get(&user_id) {
                return Ok(v.clone());
            }
        }

        let res = sqlx::query_as!(PublicKeyQuery, "SELECT user_public_key FROM aesterisk.users WHERE user_id = $1", user_id as i32).fetch_one(db::get()).await.map_err(|_| format!("User with ID {} does not exist", user_id))?;

        let cache: &WebKeyCache = self.state.web_key_cache.borrow();
        cache.insert(user_id, Arc::new(res.user_public_key.into_bytes()));
        Ok(cache.get(&user_id).ok_or("key should be in cache")?.clone())
    }

    /// Processes a WebSocket authentication packet by retrieving the user's public key and sending a handshake request to the client.
    /// 
    /// This asynchronous function first queries the public key for the user identified in the authentication packet. If successful,
    /// it forwards a handshake request to the client at the specified socket address. Any errors during the retrieval or request process
    /// are returned as an error string.
    /// 
    /// # Arguments
    /// 
    /// * `auth_packet` - The authentication packet containing the user's identifier.
    /// * `addr` - The socket address of the client.
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(())` if the handshake request is sent successfully, or an `Err` with an error message otherwise.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # async fn example() -> Result<(), String> {
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// 
    /// // Assume that `State`, `WSAuthPacket`, and `WebServer` are defined appropriately.
    /// let state = Arc::new(State::new());
    /// let server = WebServer::new(state);
    /// 
    /// let auth_packet = WSAuthPacket { user_id: 42 };
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    /// 
    /// server.handle_auth(auth_packet, addr).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn handle_auth(&self, auth_packet: WSAuthPacket, addr: SocketAddr) -> Result<(), String> {
        let key = self.query_user_public_key(auth_packet.user_id).await?;

        self.state.send_web_handshake_request(&addr, auth_packet.user_id, key)
    }

    /// Authenticates a web client using the challenge provided in the handshake response packet.
    /// 
    /// This asynchronous function extracts the challenge from the handshake response and attempts to authenticate
    /// the client by invoking the state's `authenticate_web` method with the client's socket address. A log message is
    /// recorded upon successful authentication.
    /// 
    /// # Arguments
    /// 
    /// * `handshake_reponse_packet` - The handshake response packet containing the challenge for authentication.
    /// * `addr` - The socket address of the web client.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the client fails authentication.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// 
    /// // Mock implementations for demonstration purposes.
    /// struct State;
    /// impl State {
    ///     fn authenticate_web(&self, _addr: SocketAddr, _challenge: String) -> Result<(), String> {
    ///         Ok(())
    ///     }
    /// }
    /// 
    /// struct WSHandshakeResponsePacket {
    ///     pub challenge: String,
    /// }
    /// 
    /// pub struct WebServer {
    ///     state: Arc<State>,
    /// }
    /// 
    /// impl WebServer {
    ///     pub fn new(state: Arc<State>) -> Self {
    ///         WebServer { state }
    ///     }
    /// 
    ///     pub async fn handle_handshake_response(&self, handshake_reponse_packet: WSHandshakeResponsePacket, addr: SocketAddr) -> Result<(), String> {
    ///         self.state.authenticate_web(addr, handshake_reponse_packet.challenge)?;
    ///         // Log statement omitted in this example.
    ///         Ok(())
    ///     }
    /// }
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), String> {
    ///     let state = Arc::new(State);
    ///     let server = WebServer::new(state);
    ///     let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///     let handshake_response = WSHandshakeResponsePacket { challenge: "expected_challenge".to_string() };
    ///     server.handle_handshake_response(handshake_response, addr).await?;
    ///     Ok(())
    /// }
    /// ```
    async fn handle_handshake_response(&self, handshake_reponse_packet: WSHandshakeResponsePacket, addr: SocketAddr) -> Result<(), String> {
        self.state.authenticate_web(addr, handshake_reponse_packet.challenge)?;

        info!("Authenticated");

        Ok(())
    }

    /// Processes a listen packet by forwarding its events to the client at the specified address.
    /// 
    /// This asynchronous function extracts the event list from the provided listen packet and delegates
    /// handling to the application state, which sends the events to the client identified by the socket address.
    /// 
    /// # Arguments
    /// 
    /// * `listen_packet` - A packet containing the events the client is interested in.
    /// * `addr` - The client's socket address.
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(())` if the events were successfully forwarded; otherwise, returns an `Err` with a descriptive error message.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// // Assume necessary modules and types (WebServer, WSListenPacket, and State) are imported.
    /// 
    /// # async {
    /// let state = Arc::new(/* state initialization */);
    /// let server = WebServer::new(state);
    /// let listen_packet = WSListenPacket {
    ///     events: vec!["event1".to_string(), "event2".to_string()],
    ///     ..Default::default()
    /// };
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    /// 
    /// assert!(server.handle_listen(listen_packet, addr).await.is_ok());
    /// # };
    /// ```
    async fn handle_listen(&self, listen_packet: WSListenPacket, addr: SocketAddr) -> Result<(), String> {
        // debug!("Handling listen packet: {:#?}", listen_packet);

        self.state.send_listen(addr, listen_packet.events).await
    }
}

#[async_trait]
impl Server for WebServer {
    /// Returns the binding address for the server as specified in the configuration.
    ///
    /// This function retrieves a static socket address string from the global configuration,
    /// indicating the network interface and port on which the server is set to listen.
    ///
    /// # Examples
    ///
    /// ```
    /// // This example demonstrates a dummy implementation that mirrors the behavior of `get_bind_addr`.
    /// struct DummyServer;
    ///
    /// impl DummyServer {
    ///     fn get_bind_addr(&self) -> &'static str {
    ///         "0.0.0.0:8080"
    ///     }
    /// }
    ///
    /// let server = DummyServer;
    /// assert_eq!(server.get_bind_addr(), "0.0.0.0:8080");
    /// ```
    fn get_bind_addr(&self) ->  &'static str {
        &CONFIG.sockets.web
    }

    /// Returns the static tracing name for the WebSocket server.
    ///
    /// This name is used to tag log messages, making it easier to identify events
    /// associated with the web server.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// // Assume State implements Default for demonstration purposes.
    /// use server::State;
    /// use server::web::WebServer;
    ///
    /// let state = Arc::new(State::default());
    /// let server = WebServer::new(state);
    /// assert_eq!(server.get_tracing_name(), "web");
    /// ```
    fn get_tracing_name(&self) -> &'static str {
        "web"
    }

    /// Returns the issuer identifier for the web server.
    ///
    /// This function returns a static string representing the issuer of the web server,
    /// which is utilized for logging and identification purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `web_server` is an instance of WebServer:
    /// let issuer = web_server.get_issuer();
    /// assert_eq!(issuer, "aesterisk/web");
    /// ```
    fn get_issuer(&self) ->  &'static str {
        "aesterisk/web"
    }

    /// Returns a static reference to the RSAES JWE decrypter.
    ///
    /// This decrypter, provided by the `josekit` library, is used by the WebSocket server to decrypt
    /// incoming JWE messages. Being a static instance, it ensures that the same decrypter configuration
    /// is used consistently throughout the application.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming a `WebServer` instance named `server` has been created.
    /// let decrypter = server.get_decrypter();
    /// // Use `decrypter` to handle JWE decryption tasks.
    /// ```
    fn get_decrypter(&self) ->  &'static josekit::jwe::alg::rsaes::RsaesJweDecrypter {
        &DECRYPTER
    }

    /// Accepts a new web client connection by registering its socket address with the server state.
    /// 
    /// This method adds the connection's address and its associated transmitter channel to the state,
    /// allowing the server to send messages to the client.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// 
    /// // Assume State, WebServer, and Tx are properly defined and imported.
    /// async fn example() -> Result<(), String> {
    ///     let state = Arc::new(State::new());
    ///     let server = WebServer::new(state);
    ///     let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
    ///     let tx = create_tx(); // Replace with actual transmitter channel creation for Tx
    ///     server.on_accept(addr, tx).await
    /// }
    /// ```
    async fn on_accept(&self, addr: SocketAddr, tx: Tx) -> Result<(), String> {
        self.state.add_web(addr, tx);

        Ok(())
    }

    /// Disconnects a web client by removing it from the server's state.
    ///
    /// This asynchronous method is called when a client disconnects, ensuring that the client's
    /// connection information is removed from the application state.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// use tokio_test::block_on;
    ///
    /// // Dummy State implementation for demonstration purposes.
    /// struct State;
    /// impl State {
    ///     async fn remove_web(&self, _addr: SocketAddr) -> Result<(), String> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// // Minimal WebServer struct with the `on_disconnect` method.
    /// struct WebServer {
    ///     state: Arc<State>,
    /// }
    ///
    /// impl WebServer {
    ///     async fn on_disconnect(&self, addr: SocketAddr) -> Result<(), String> {
    ///         self.state.remove_web(addr).await
    ///     }
    /// }
    ///
    /// let state = Arc::new(State);
    /// let server = WebServer { state };
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    /// block_on(server.on_disconnect(addr)).unwrap();
    /// ```
    async fn on_disconnect(&self, addr: SocketAddr) -> Result<(), String> {
        self.state.remove_web(addr).await
    }

    /// Handles a decryption error by disconnecting the web client at the specified socket address.
    ///
    /// This asynchronous function delegates the disconnection process to the application's state management.
    /// It is typically invoked when a decryption failure occurs, returning an error if the disconnection process
    /// fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// // Assume `server` is a valid instance of WebServer.
    /// # async fn example() -> Result<(), String> {
    /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    /// server.on_decrypt_error(addr).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn on_decrypt_error(&self, addr: SocketAddr) -> Result<(), String> {
        self.state.disconnect_web(addr)
    }

    #[instrument("web", skip(self, packet))]
    /// Processes an incoming WebSocket packet by dispatching it to a specific handler based on its identifier.
    /// 
    /// The function examines the packet's ID and:
    /// - Calls `handle_auth` for authentication packets (`ID::WSAuth`)
    /// - Calls `handle_handshake_response` for handshake response packets (`ID::WSHandshakeResponse`)
    /// - Calls `handle_listen` for listen events (`ID::WSListen`)
    /// 
    /// Packets with any other identifier result in an error.
    /// 
    /// # Panics
    /// 
    /// This function panics if packet parsing fails.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # async fn example() -> Result<(), String> {
    /// use std::net::SocketAddr;
    /// // Initialize shared state and create a WebServer instance (state initialization omitted).
    /// let state = /* state initialization */; 
    /// let server = WebServer::new(state);
    /// let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
    /// 
    /// // Create a WSAuth packet (payload omitted for brevity).
    /// let packet = Packet::new(ID::WSAuth, vec![]);
    /// 
    /// // Process the packet.
    /// server.on_packet(packet, addr).await?;
    /// # Ok(())
    /// # }
    /// ```
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
