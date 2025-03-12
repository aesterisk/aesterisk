use packet::{server_daemon::{auth_response::SDAuthResponsePacket, handshake_request::SDHandshakeRequestPacket, init_data::SDInitDataPacket, listen::SDListenPacket}, ID};
use tracing::debug;

use crate::encryption;

mod auth;
mod handshake;
mod init_data;
mod listen;

/// Decrypts, parses and handles an incoming packet
pub async fn handle(msg: String) -> Result<(), String> {
    let packet = encryption::decrypt_packet(&msg).await?;

    debug!("Received Packet {:?}", packet.id);

    match packet.id {
        ID::SDAuthResponse => {
            auth::handle(SDAuthResponsePacket::parse(packet).ok_or("Could not parse SDAuthResponsePacket")?).await
        }
        ID::SDHandshakeRequest => {
            handshake::handle(SDHandshakeRequestPacket::parse(packet).ok_or("Could not parse SDHandshakeRequestPacket")?).await
        }
        ID::SDListen => {
            listen::handle(SDListenPacket::parse(packet).ok_or("Could not parse SDListenPacket")?).await
        }
        ID::SDInitData => {
            init_data::handle(SDInitDataPacket::parse(packet).ok_or("Could not parse SDInitDataPacket")?).await
        }
        _ => {
            Err(format!("Should not receive [A*|D*|SA] packet: {:?}", packet.id))
        }
    }
}
