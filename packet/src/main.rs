use aesterisk_packet::{events::{EventData, EventType, ListenEvent, NodeStatusEvent}, server_web::event::SWEventPacket, web_server::listen::WSListenPacket};
use uuid::uuid;

fn main() {
    let id = uuid!("422c01f6-dc04-42d2-98ca-a3ea05a0b505");

    let packet = WSListenPacket {
        events: vec![ListenEvent {
            event: EventType::NodeStatus,
            daemons: vec![id],
        }],
    }.to_packet().unwrap();

    println!("Listen: {}", packet.to_string());

    let packet2 = SWEventPacket {
        event: EventData::NodeStatus(NodeStatusEvent {
            online: true,
        }),
        daemon: id
    }.to_packet().unwrap();

    println!(" Event: {}", packet2.to_string());
}
