use packet::server_daemon::sync::SDSyncPacket;
use tracing::{debug, info};

use crate::docker;

pub async fn handle(sync_packet: SDSyncPacket) -> Result<(), String> {
    info!("Syncing data from server with Docker");

    for nw in sync_packet.networks {
        debug!("Checking network {}", nw.id);
        if !docker::network::network_exists(nw.id).await? {
            debug!("Creating network {}", nw.id);
            let id = docker::network::create_network(nw.id, nw.subnet).await?;
            debug!("Created network ({})", id);
        }
    }

    Ok(())
}
