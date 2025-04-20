use packet::server_daemon::sync::SDSyncPacket;
use tracing::{debug, error, info};

use crate::{docker, services::{self, server_status}};

pub async fn handle(sync_packet: SDSyncPacket) -> Result<(), String> {
    info!("Syncing data from server with Docker");

    debug!("Syncing networks...");
    for nw in sync_packet.networks {
        debug!("  Checking network {}", nw.id);
        if !docker::network::network_exists(nw.id).await? {
            debug!("    Creating network {}", nw.id);
            let id = docker::network::create_network(nw.id, nw.subnet).await?;
            debug!("    Created network ({})", id);
        }
    }

    debug!("Stopping running stats services...");
    server_status::stop_services().await?;

    debug!("Syncing servers...");
    for server in sync_packet.servers {
        let id = server.id;

        debug!("  Checking server {}", id);
        if !docker::server::server_exists(id).await? {
            debug!("    Creating server {}", id);
            let docker_id = docker::server::create_server(server).await?;
            debug!("    Created server ({})", docker_id);
        }

        debug!("  Starting stats service");
        tokio::spawn(async move {
            match server_status::start(id).await {
                Ok(_) => (),
                Err(e) => error!("Error in server stats service: {}", e),
            };

            debug!("Stats service for server {} has stopped", id);
        });
    }

    Ok(())
}
