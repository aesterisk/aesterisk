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

    /// Start the serer.
    async fn start(self: Arc<Self>) {
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

    /// Handle a TCP connection.
    async fn accept_connection(self: Arc<Self>, raw_stream: TcpStream, addr: SocketAddr) -> Result<(), String> {
        debug!("Accepted TCP connection");

        let stream = tokio_tungstenite::accept_async(raw_stream).await.map_err(|e| format!("Could not accept connection: {}", self.error_to_string(e)))?;
        let (write, read) = stream.split();

        let (tx, rx) = unbounded();

        self.on_accept(addr, tx).instrument(Span::current()).await?;

        self.handle_client(write, read, addr, rx).await?;

        Ok(())
    }

    /// Handle a WebSocket connection.
    async fn handle_client(self: Arc<Self>, write: SplitSink<WebSocketStream<TcpStream>, Message>, read: SplitStream<WebSocketStream<TcpStream>>, addr: SocketAddr, rx: Rx) -> Result<(), String> {
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

    /// Handle a packet.
    async fn handle_packet(self: Arc<Self>, msg: String, addr: SocketAddr) -> Result<(), String> {
        let on_err = async || {
            self.on_decrypt_error(addr).await
        };

        let packet = encryption::decrypt_packet(&msg, self.get_decrypter(), self.get_issuer(), Some(on_err)).await?;

        self.on_packet(packet, addr).instrument(Span::current()).await
    }

    /// Convert a `tungstenite::Error` to a `String` in a pretty format.
    fn error_to_string(&self, e: tungstenite::Error) -> String {
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
