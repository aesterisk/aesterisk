use std::{collections::HashMap, net::SocketAddr, sync::{Arc, Mutex}};

use futures_channel::mpsc::{self, unbounded};
use futures_util::{future, pin_mut, stream::{SplitSink, SplitStream}, FutureExt, StreamExt, TryStreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite::{self, Message}, WebSocketStream};

use packet::{daemon_server::{auth::DSAuthPacket, event::DSEventPacket}, Packet, ID};
use tracing::{debug, error, info, span, Level};

use crate::config::Config;

struct DaemonSocket {
    tx: Tx,
}

type Tx = mpsc::UnboundedSender<Message>;
type Rx = mpsc::UnboundedReceiver<Message>;
type ChannelMap = Arc<Mutex<HashMap<SocketAddr, DaemonSocket>>>;

pub async fn start(config: &Config, private_key: &josekit::jwk::Jwk) {
    let try_socket = TcpListener::bind(&config.sockets.daemon).await;
    let listener = match try_socket {
        Ok(listener) => listener,
        Err(e) => {
            error!("Error binding to socket: {}", e);
            return;
        },
    };

    info!("Listening on: {}", &config.sockets.daemon);

    let channel_map = ChannelMap::new(Mutex::new(HashMap::new()));

    loop {
        let conn = listener.accept().await;

        match conn {
            Ok((stream, addr)) => {
                tokio::spawn(accept_connection(stream, addr, channel_map.clone()).then(|res| match res {
                    Ok(_) => future::ready(()),
                    Err(e) => {
                        error!("Error in connection: {}", e);
                        future::ready(())
                    },
                }));
            },
            Err(e) => {
                error!("Error accepting connection: {}", e.kind());
            },
        }
    }
}

// TODO: move to shared utils module
fn error_to_string(e: tungstenite::Error) -> String {
    match e {
        tungstenite::Error::Utf8 => format!("Error in UTF-8 encoding"),
        tungstenite::Error::Io(e) => format!("IO error ({})", e.kind()),
        tungstenite::Error::Tls(_) => format!("TLS error"),
        tungstenite::Error::Url(_) => format!("Invalid URL"),
        tungstenite::Error::Http(_) => format!("HTTP error"),
        tungstenite::Error::HttpFormat(_) => format!("HTTP format error"),
        tungstenite::Error::Capacity(_) => format!("Buffer capacity exhausted"),
        tungstenite::Error::Protocol(_) => format!("Protocol violation"),
        tungstenite::Error::AlreadyClosed => format!("Connection already closed"),
        tungstenite::Error::AttackAttempt => format!("Attack attempt detected"),
        tungstenite::Error::WriteBufferFull(_) => format!("Write buffer full"),
        tungstenite::Error::ConnectionClosed => format!("Connection closed"),
    }
}

#[tracing::instrument(name = "daemon", skip(raw_stream, channel_map), fields(%addr))]
async fn accept_connection(raw_stream: TcpStream, addr: SocketAddr, channel_map: ChannelMap) -> Result<(), String> {
    info!("Accepted TCP connection");

    let stream = tokio_tungstenite::accept_async(raw_stream).await.map_err(|e| format!("Could not accept connection: {}", error_to_string(e)))?;

    let (write, read) = stream.split();

    let (tx, rx) = unbounded();
    channel_map.lock().map_err(|_| "channel_map has been poisoned")?.insert(addr, DaemonSocket { tx });

    handle_client(write, read, addr, rx, channel_map).await?;

    Ok(())
}

async fn handle_client(write: SplitSink<WebSocketStream<TcpStream>, Message>, read: SplitStream<WebSocketStream<TcpStream>>, addr: SocketAddr, rx: Rx, channel_map: ChannelMap) -> Result<(), String> {
    info!("Established WebSocket connection");

    let incoming = read.try_filter(|msg| future::ready(msg.is_text())).for_each(|msg| async {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                error!("Error reading message: {}", error_to_string(e));
                return;
            },
        };

        let text = match msg.into_text() {
            Ok(text) => text,
            Err(e) => {
                error!("Error reading message: {}", error_to_string(e));
                return;
            }
        };

        tokio::spawn(handle_packet(text, addr).then(|e| match e {
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

    channel_map.lock().map_err(|_| "channel_map has been poisoned")?.remove(&addr);
    info!("Disconnected");

    Ok(())
}

#[tracing::instrument(name = "daemon", skip(msg), fields(%addr))]
async fn handle_packet(msg: String, addr: SocketAddr) -> Result<(), String> {
    let try_packet = Packet::from_str(&msg);

    let packet = match try_packet {
        Some(packet) => packet,
        None => {
            return Err(format!("Error parsing packet: \"{}\"", msg));
        }
    };

    match packet.id {
        ID::DSAuth => {
            handle_auth(DSAuthPacket::parse(packet).ok_or("DSAuthPacket should be Some")?, addr).await
        },
        ID::DSEvent => {
            handle_event(DSEventPacket::parse(packet).ok_or("DSEventPacket should be Some")?, addr).await
        },
        _ => {
            Err(format!("Should not receive [AS]* packet: {:?}", packet.id))
        },
    }
}

async fn handle_auth(auth_packet: DSAuthPacket, addr: SocketAddr) -> Result<(), String> {
    debug!("DSAuthPacket: {:?}", auth_packet);

    Ok(())
}

async fn handle_event(event_packet: DSEventPacket, addr: SocketAddr) -> Result<(), String> {
    debug!("DSEventPacket: {:?}", event_packet);

    Ok(())
}
