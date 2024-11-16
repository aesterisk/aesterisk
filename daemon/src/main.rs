use std::{sync::{Arc, Mutex}, thread, time::Duration};

use futures_channel::mpsc::{self, unbounded};
use futures_util::{future, join, pin_mut, StreamExt, TryStreamExt};
use packet::{daemon_server::auth::DSAuthPacket, server_daemon::{auth_response::SDAuthResponsePacket, listen::SDListenPacket}, Packet, ID};
use tokio_tungstenite::tungstenite::Message;

type Rx = mpsc::UnboundedReceiver<Message>;
type Tx = mpsc::UnboundedSender<Message>;
type Sender = Arc<Mutex<Option<Tx>>>;

#[tokio::main]
async fn main() {
    let sender = Arc::new(Mutex::new(None));

    let server_connector_handle = tokio::spawn(start_server_connector(sender.clone()));

    join!(server_connector_handle).0.expect("failed to join handle");
}

async fn start_server_connector(sender: Sender) {
    loop {
        let (tx, rx) = unbounded();
        sender.lock().expect("failed to gain lock").replace(tx);

        let _ = join!(tokio::spawn(connect_to_server(rx, sender.clone())));

        thread::sleep(Duration::from_secs(1));
    }
}

async fn connect_to_server(rx: Rx, sender: Sender) {
    let (stream, _) = tokio_tungstenite::connect_async("ws://127.0.0.1:31304").await.expect("failed to connect");

    println!("Connected to server");

    let (write, read) = stream.split();

    tokio::spawn(handle_connection(sender.clone()));

    let incoming = read.try_filter(|msg| future::ready(msg.is_text())).for_each(|msg| async {
        let msg = msg.expect("message should be ok").into_text().expect("message should be of type text");
        println!("Message: {}", msg);
        tokio::spawn(handle_packet(msg));
    });

    let outgoing = rx.map(Ok).forward(write);

    pin_mut!(incoming, outgoing);
    future::select(incoming, outgoing).await;

    println!("Disconnected from server");
}

async fn handle_connection(sender: Sender) {
    let auth_packet = DSAuthPacket {
        id: 1,
        token: String::from("hi"),
    };

    let auth_packet_data = auth_packet.to_string().expect("packet should be serializeable");

    sender.lock().expect("lock should not be poisoned").as_ref().expect("sender should be available").unbounded_send(Message::Text(auth_packet_data)).expect("message should get sent");
}

async fn handle_packet(msg: String) {
    let try_packet = Packet::from_str(&msg);

    if try_packet.is_none() {
        return;
    }

    let packet = try_packet.expect("packet should be some");

    println!("Packet:\n{:#?}", packet);

    match packet.id {
        ID::SDAuthResponse => {
            handle_auth_response(SDAuthResponsePacket::parse(packet).expect("SDAuthResponsePacket should be Some")).await;
        }
        ID::SDListen => {
            handle_listen(SDListenPacket::parse(packet).expect("SDListenPacket should be Some")).await;
        }
        _ => {
            eprintln!("(E) Should not receive [A*|D*|SA] packet: {:?}", packet.id);
        }
    }
}

async fn handle_auth_response(auth_response_packet: SDAuthResponsePacket) {
    println!("Auth Response:\n{:#?}", auth_response_packet);
}

async fn handle_listen(listen_packet: SDListenPacket) {
    println!("Listen:\n{:#?}", listen_packet);
}
