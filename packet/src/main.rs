use packet::{app_server::listen::ASListenPacket, Event, NodeStatus};

fn main() {
    let packet = ASListenPacket {
        events: vec![Event::NodesList(vec![NodeStatus {
            id: 1,
            status: true,
        }])],
    };

    println!("{}", packet.to_string().unwrap());
}
