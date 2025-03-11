use packet::server_daemon::auth_response::SDAuthResponsePacket;
use tracing::info;

/// Handles the SDAuthResponsePacket
pub async fn handle(auth_response_packet: SDAuthResponsePacket) -> Result<(), String> {
    if !auth_response_packet.success {
        return Err("Unsuccessful auth response".to_string());
    }

    info!("Authenticated");

    Ok(())
}

