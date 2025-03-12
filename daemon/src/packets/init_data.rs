use packet::server_daemon::init_data::SDInitDataPacket;
use tracing::debug;

pub async fn handle(init_data_packet: SDInitDataPacket) -> Result<(), String> {
    debug!("Received InitDataPacket: {:#?}", init_data_packet);

    Ok(())
}
