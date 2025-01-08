use aesterisk_packet::{events::{EventData, EventType, ListenEvent, NodeStats, NodeStatusEvent}, server_web::event::SWEventPacket, web_server::listen::WSListenPacket};
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
            stats: Some(NodeStats {
                used_memory: 16.2,
                total_memory: 32.0,
                cpu: 56.0,
                used_storage: 180.4,
                total_storage: 256.0,
            })
        }),
        daemon: id
    }.to_packet().unwrap();

    println!(" Event: {}", packet2.to_string());
}
