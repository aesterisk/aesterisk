use packet::server_daemon::listen::SDListenPacket;

use crate::LISTENS;

pub async fn handle(listen_packet: SDListenPacket) -> Result<(), String> {
    *LISTENS.write().await = listen_packet.events;

    Ok(())
}
