use std::{collections::HashMap, net::SocketAddr, sync::{Arc, Mutex}};

use futures_channel::mpsc::{self, unbounded};
use futures_util::{future, pin_mut, stream::{SplitSink, SplitStream}, StreamExt, TryStreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use packet::{daemon_server::{auth::DSAuthPacket, event::DSEventPacket}, Packet, ID};

use crate::config::Config;

struct DaemonSocket {
    tx: Tx,
}

type Tx = mpsc::UnboundedSender<Message>;
type Rx = mpsc::UnboundedReceiver<Message>;
type ChannelMap = Arc<Mutex<HashMap<SocketAddr, DaemonSocket>>>;

pub async fn start(config: &Config, private_key: &josekit::jwk::Jwk) {
    let try_socket = TcpListener::bind(&config.sockets.daemon).await;
    let listener = try_socket.expect("call to bind should be ok");

    println!("  (Daemon) Listening on: {}", &config.sockets.daemon);

    let channel_map = ChannelMap::new(Mutex::new(HashMap::new()));

    loop {
        let conn = listener.accept().await;

        match conn {
            Ok((stream, addr)) => {
                tokio::spawn(accept_connection(stream, addr, channel_map.clone()));
            }
            Err(e) => {
                eprintln!("E (Daemon) Error: {}", e);
                break;
            }
        }
        while let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(accept_connection(stream, addr, channel_map.clone()));
        }
    }

    println!("W (Daemon) Shutting down server");
}

async fn accept_connection(raw_stream: TcpStream, addr: SocketAddr, channel_map: ChannelMap) {
    println!("  (Daemon) [{}] Accepted TCP connection", addr);

    let stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("handshake should be established");

    let (write, read) = stream.split();

    let (tx, rx) = unbounded();
    channel_map.lock().expect("lock should not be poisoned").insert(addr, DaemonSocket { tx });

    handle_client(write, read, addr, rx, channel_map).await;
}

async fn handle_client(write: SplitSink<WebSocketStream<TcpStream>, Message>, read: SplitStream<WebSocketStream<TcpStream>>, addr: SocketAddr, rx: Rx, channel_map: ChannelMap) {
    println!("  (Daemon) [{}] Established WebSocket connection", addr);

    let incoming = read.try_filter(|msg| future::ready(msg.is_text())).for_each(|msg| async {
        let msg = msg.expect("message should be ok").into_text().expect("message should be of type text");
        tokio::spawn(handle_packet(msg, addr));
    });

    let outgoing = rx.map(Ok).forward(write);

    pin_mut!(incoming, outgoing);
    future::select(incoming, outgoing).await;

    channel_map.lock().expect("failed to gain lock").remove(&addr);
    println!("  (Daemon) [{}] Disconnected", addr);
}

async fn handle_packet(msg: String, addr: SocketAddr) {
    let try_packet = Packet::from_str(&msg);

    if try_packet.is_none() {
        return;
    }

    let packet = try_packet.expect("packet should be some");

    match packet.id {
        ID::DSAuth => {
            handle_auth(DSAuthPacket::parse(packet).expect("DSAuthPacket should be Some"), addr).await;
        }
        ID::DSEvent => {
            handle_event(DSEventPacket::parse(packet).expect("DSEventPacket should be Some"), addr).await;
        }
        _ => {
            eprintln!("E (Daemon) Should not receive [AS]* packet: {:?}", packet.id);
        }
    }
}

async fn handle_auth(auth_packet: DSAuthPacket, addr: SocketAddr) {
    println!("  (Daemon) [{}] {:?}", addr, auth_packet);
}

async fn handle_event(event_packet: DSEventPacket, addr: SocketAddr) {
    println!("  (Daemon) [{}] {:?}", addr, event_packet);
}
