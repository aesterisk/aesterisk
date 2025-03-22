use packet::server_daemon::init_data::SDInitDataPacket;
use tracing::{debug, info};

use crate::docker;

pub async fn handle(init_data_packet: SDInitDataPacket) -> Result<(), String> {
    info!("Syncing data from server with Docker");

    for nw in init_data_packet.networks {
        debug!("Checking network {}", nw.id);
        if !docker::network::network_exists(nw.id).await? {
            debug!("Creating network {}", nw.id);
            let id = docker::network::create_network(nw.id, nw.subnet).await?;
            debug!("Created network ({})", id);
        }
    }

    Ok(())
}
