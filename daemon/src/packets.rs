use packet::{server_daemon::{auth_response::SDAuthResponsePacket, handshake_request::SDHandshakeRequestPacket, listen::SDListenPacket}, ID};
use tracing::debug;

use crate::encryption;

mod auth;
mod handshake;
mod listen;

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
        _ => {
            Err(format!("Should not receive [A*|D*|SA] packet: {:?}", packet.id))
        }
    }
}
