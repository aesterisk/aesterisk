use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use futures_channel::mpsc::unbounded;
use futures_util::{future, pin_mut, stream::{SplitSink, SplitStream}, StreamExt, TryStreamExt};
use josekit::jwe::alg::rsaes::RsaesJweDecrypter;
use packet::Packet;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite::{self, Message}, WebSocketStream};
use tracing::{debug, error, info, span, Level, Span};
use tracing_futures::Instrument;

use crate::{encryption, state::{Rx, Tx}};

/// The main `Server` trait, which handles WebSocket connections, decryption and parsing of
/// packets.
#[async_trait]
pub trait Server: Send + Sync + 'static {

    /// Return the name to use with `tracing` logs
    fn get_tracing_name(&self) -> &'static str;
    /// Return the address to bind to
    fn get_bind_addr(&self) -> &'static str;
    /// Return the decrypter to use when decrypting packets
    fn get_decrypter(&self) -> &'static RsaesJweDecrypter;
    /// Return the issuer to use when decrypting packets
    fn get_issuer(&self) -> &'static str;

    /// Called when a new connection is accepted
    async fn on_accept(&self, addr: SocketAddr, tx: Tx) -> Result<(), String>;
    /// Called when a connection is disconnected
    async fn on_disconnect(&self, addr: SocketAddr) -> Result<(), String>;
    /// Called when a packet could not be decrypted
    async fn on_decrypt_error(&self, addr: SocketAddr) -> Result<(), String>;
    /// Called when a packet is received
    async fn on_packet(&self, packet: Packet, addr: SocketAddr) -> Result<(), String>;

    /// Starts the server by binding to a designated address and processing incoming connections asynchronously.
    /// 
    /// This method binds to the address returned by `get_bind_addr` and enters an infinite loop to accept new TCP connections.
    /// Each accepted connection is upgraded and handled in a dedicated asynchronous task via `accept_connection`. Any errors 
    /// encountered during binding or while accepting connections are logged rather than propagated.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use std::sync::Arc;
    /// 
    /// // Assume MyServer is a type that implements the Server trait.
    /// let server = Arc::new(MyServer::new());
    /// 
    /// #[tokio::main]
    /// async fn main() {
    ///     server.start().await;
    /// }
    /// ```    async fn start(self: Arc<Self>) {
        let tracing_name = self.as_ref().get_tracing_name();
        async move {
            let try_socket = TcpListener::bind(self.get_bind_addr()).await;
            let listener = match try_socket {
                Ok(listener) => listener,
                Err(e) => {
                    error!("Error binding to socket: {}", e);
                    return;
                }
            };

            info!("Listening on: {}", self.get_bind_addr());

            loop {
                let conn = listener.accept().await;

                match conn {
                    Ok((stream, addr)) => {
                        let self_cloned = Arc::clone(&self);
                        tokio::spawn(async move {
                            match self_cloned.accept_connection(stream, addr).await {
                                Ok(_) => future::ready(()),
                                Err(e) => {
                                    error!("Error in connection: {}", e);
                                    future::ready(())
                                },
                            }
                        }.instrument(span!(Level::TRACE, "client", "addr" = %addr)));
                    }
                    Err(e) => {
                        error!("Error in connection: {}", e);
                    }
                }
            }
        }.instrument(span!(Level::TRACE, "server", "type" = tracing_name)).await
    }

    /// Upgrades a raw TCP connection to a WebSocket and initiates client handling.
    ///
    /// This asynchronous function accepts a raw TCP stream and a client address, upgrades the connection to a WebSocket
    /// using `tokio_tungstenite::accept_async`, and splits the stream into reading and writing halves. It then notifies the server
    /// of the new connection via the `on_accept` callback and delegates further client management to `handle_client`.
    ///
    /// # Errors
    ///
    /// Returns an error as a `String` if the WebSocket upgrade fails or if handling the client session encounters an error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// use tokio::net::TcpListener;
    /// use your_crate::server::YourServerImpl; // Replace with your implementation of the Server trait.
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr: SocketAddr = "127.0.0.1:9001".parse().unwrap();
    ///     let listener = TcpListener::bind(addr).await.unwrap();
    ///
    ///     let (tcp_stream, client_addr) = listener.accept().await.unwrap();
    ///     let server = Arc::new(YourServerImpl::default());
    ///
    ///     server.accept_connection(tcp_stream, client_addr).await.unwrap();
    /// }
    /// ```    async fn accept_connection(self: Arc<Self>, raw_stream: TcpStream, addr: SocketAddr) -> Result<(), String> {
        debug!("Accepted TCP connection");

        let stream = tokio_tungstenite::accept_async(raw_stream).await.map_err(|e| format!("Could not accept connection: {}", self.error_to_string(e)))?;
        let (write, read) = stream.split();

        let (tx, rx) = unbounded();

        self.on_accept(addr, tx).instrument(Span::current()).await?;

        self.handle_client(write, read, addr, rx).await?;

        Ok(())
    }

    /// Handles a WebSocket client connection by concurrently processing incoming and outgoing messages.
    ///
    /// Incoming messages are filtered to only process text messages. Each text message is converted to a string
    /// and then processed asynchronously by invoking `handle_packet`. Outgoing messages received via a channel
    /// are forwarded to the client. When either the incoming or outgoing stream completes, the connection is
    /// terminated and `on_disconnect` is called.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// use tokio::net::TcpStream;
    /// use tokio::sync::mpsc;
    /// use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
    /// use futures::{SinkExt, StreamExt};
    ///
    /// // Dummy implementations for demonstration purposes.
    /// struct DummyServer;
    ///
    /// #[allow(unused)]
    /// impl DummyServer {
    ///     async fn handle_packet(self: Arc<Self>, _msg: String, _addr: SocketAddr) -> Result<(), String> {
    ///         Ok(())
    ///     }
    ///
    ///     async fn on_disconnect(&self, _addr: SocketAddr) -> Result<(), String> {
    ///         Ok(())
    ///     }
    ///
    ///     fn error_to_string(&self, _e: tungstenite::Error) -> String {
    ///         "error".into()
    ///     }
    /// }
    ///
    /// // We assume handle_client is implemented on DummyServer similarly to the actual server.
    /// async fn demo_handle_client() -> Result<(), String> {
    ///     // Create dummy channels and socket address.
    ///     let (tx, rx) = mpsc::channel(10);
    ///     let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///
    ///     // For demonstration, we are using unimplemented!() for the WebSocket stream parts.
    ///     let write: futures::sink::Sink<Message, Error = tungstenite::Error> = unimplemented!();
    ///     let read: futures::stream::Stream<Item = Result<Message, tungstenite::Error>> = unimplemented!();
    ///
    ///     let server = Arc::new(DummyServer);
    ///
    ///     // Call handle_client to process the connection.
    ///     server.handle_client(write, read, addr, rx).await
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// #     let _ = demo_handle_client().await;
    /// # }
    /// ```    async fn handle_client(self: Arc<Self>, write: SplitSink<WebSocketStream<TcpStream>, Message>, read: SplitStream<WebSocketStream<TcpStream>>, addr: SocketAddr, rx: Rx) -> Result<(), String> {
        debug!("Established WebSocket connection");

        let incoming = read.try_filter(|msg| future::ready(msg.is_text())).for_each(|msg| async {
            let msg = match msg {
                Ok(msg) => msg,
                Err(e) => {
                    error!("Error reading message: {}", self.error_to_string(e));
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

            let self_cloned = Arc::clone(&self);
            tokio::spawn(async move {
                match self_cloned.handle_packet(text, addr).await {
                    Ok(_) => future::ready(()),
                    Err(e) => {
                        error!("Error handling packet: {}", e);
                        future::ready(())
                    },
                }
            });
        });

        let outgoing = rx.map(Ok).forward(write);

        pin_mut!(incoming, outgoing);
        future::select(incoming, outgoing).await;

        let res = self.on_disconnect(addr).instrument(Span::current()).await;

        info!("Disconnected");

        res
    }

    /// Processes an incoming encrypted packet from a client.
    ///
    /// This asynchronous function attempts to decrypt the provided message using the server's configured decrypter and issuer.
    /// If decryption fails, it invokes the decryption error handler before returning an error.
    /// On successful decryption, it forwards the resulting packet to the packet handler for further processing.
    ///
    /// # Arguments
    ///
    /// * `msg` - The encrypted message received from the client.
    /// * `addr` - The client's socket address.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the packet is successfully decrypted and handled; otherwise returns an error message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// use server::Server; // Adjust import as needed
    /// use async_trait::async_trait;
    ///
    /// struct MyServer;
    ///
    /// #[async_trait]
    /// impl Server for MyServer {
    ///     // Implement all required trait methods.
    ///     async fn on_decrypt_error(&self, _addr: SocketAddr) -> Result<(), String> {
    ///         Ok(())
    ///     }
    ///
    ///     async fn on_packet(&self, _packet: Packet, _addr: SocketAddr) -> Result<(), String> {
    ///         Ok(())
    ///     }
    ///
    ///     // Stub implementations for abstract methods.
    ///     fn get_decrypter(&self) -> &Decrypter { unimplemented!() }
    ///     fn get_issuer(&self) -> &str { unimplemented!() }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let server = Arc::new(MyServer);
    ///     let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    ///     let encrypted_msg = "encrypted message".to_string();
    ///
    ///     let result = server.handle_packet(encrypted_msg, addr).await;
    ///     assert!(result.is_ok(), "Packet handling failed");
    /// }
    /// ```    async fn handle_packet(self: Arc<Self>, msg: String, addr: SocketAddr) -> Result<(), String> {
        let on_err = async || {
            self.on_decrypt_error(addr).await
        };

        let packet = encryption::decrypt_packet(&msg, self.get_decrypter(), self.get_issuer(), Some(on_err)).await?;

        self.on_packet(packet, addr).instrument(Span::current()).await
    }

    /// Converts a `tungstenite::Error` into a human-readable string.
    ///
    /// This function matches on each variant of `tungstenite::Error` and returns
    /// a descriptive message suitable for logging or debugging purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// use tungstenite::Error;
    ///
    /// // Dummy struct mimicking a server that implements `error_to_string`.
    /// struct DummyServer;
    ///
    /// impl DummyServer {
    ///     fn error_to_string(&self, e: Error) -> String {
    ///         match e {
    ///             Error::Utf8 => "Error in UTF-8 encoding".into(),
    ///             Error::Io(e) => format!("IO error ({})", e.kind()),
    ///             Error::Tls(_) => "TLS error".into(),
    ///             Error::Url(_) => "Invalid URL".into(),
    ///             Error::Http(_) => "HTTP error".into(),
    ///             Error::HttpFormat(_) => "HTTP format error".into(),
    ///             Error::Capacity(_) => "Buffer capacity exhausted".into(),
    ///             Error::Protocol(_) => "Protocol violation".into(),
    ///             Error::AlreadyClosed => "Connection already closed".into(),
    ///             Error::AttackAttempt => "Attack attempt detected".into(),
    ///             Error::WriteBufferFull(_) => "Write buffer full".into(),
    ///             Error::ConnectionClosed => "Connection closed".into(),
    ///         }
    ///     }
    /// }
    ///
    /// let server = DummyServer;
    /// let err_msg = server.error_to_string(Error::Utf8);
    /// assert_eq!(err_msg, "Error in UTF-8 encoding");
    /// ```    fn error_to_string(&self, e: tungstenite::Error) -> String {
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

}
