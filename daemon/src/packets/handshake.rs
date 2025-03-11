use packet::{daemon_server::handshake_response::DSHandshakeResponsePacket, server_daemon::handshake_request::SDHandshakeRequestPacket};
use tokio_tungstenite::tungstenite::Message;

use crate::{encryption, SENDER};

/// Handles the SDHandshakeRequestPacket
pub async fn handle(handshake_request_packet: SDHandshakeRequestPacket) -> Result<(), String> {
    SENDER.lock().await.as_ref().ok_or("sender is not available")?.unbounded_send(
        Message::Text(
            encryption::encrypt_packet(
                DSHandshakeResponsePacket {
                    challenge: handshake_request_packet.challenge,
                }.to_packet()?,
            )?
        )
    ).map_err(|e| format!("Could not send packet: {}", e))?;

    Ok(())
}

