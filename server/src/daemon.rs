use std::{borrow::Borrow, net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use josekit::jwe::alg::rsaes::{RsaesJweDecrypter, RsaesJweEncrypter};
use packet::{daemon_server::{auth::DSAuthPacket, event::DSEventPacket, handshake_response::DSHandshakeResponsePacket}, Packet, ID};
use sqlx::types::Uuid;
use tracing::{info, instrument};

use crate::{config::CONFIG, db, encryption::DECRYPTER, server::Server, state::{DaemonKeyCache, State, Tx}};

pub struct DaemonHandshake {
    pub daemon_uuid: Uuid,
    pub encrypter: RsaesJweEncrypter,
    pub challenge: String,
}

pub struct DaemonSocket {
    pub tx: Tx,
    pub handshake: Option<DaemonHandshake>,
}

pub struct DaemonServer {
    state: Arc<State>,
}

struct PublicKeyQuery {
    node_public_key: String,
}

impl DaemonServer {
    pub fn new(state: Arc<State>) -> Self {
        Self {
            state
        }
    }

    async fn query_user_public_key(&self, daemon_uuid: &Uuid) -> Result<Arc<Vec<u8>>, String> {
        {
            let cache: &DaemonKeyCache = self.state.daemon_key_cache.borrow();
            if let Some(v) = cache.get(daemon_uuid) {
                return Ok(v.clone());
            }
        }

        let res = sqlx::query_as!(PublicKeyQuery, "SELECT node_public_key FROM aesterisk.nodes WHERE node_uuid = $1", daemon_uuid).fetch_one(db::get()).await.map_err(|_| format!("Node with UUID {} does not exist", &daemon_uuid))?;

        let cache: &DaemonKeyCache = self.state.daemon_key_cache.borrow();
        cache.insert(*daemon_uuid, Arc::new(res.node_public_key.into_bytes()));
        Ok(cache.get(daemon_uuid).ok_or("key should be in cache")?.clone())
    }

    async fn handle_auth(&self, auth_packet: DSAuthPacket, addr: SocketAddr) -> Result<(), String> {
        let uuid = Uuid::parse_str(&auth_packet.daemon_uuid).map_err(|_| "Could not parse UUID")?;
        let key = self.query_user_public_key(&uuid).await?;

        self.state.send_daemon_handshake_request(addr, uuid, key).await
    }

    async fn handle_handshake_response(&self, handshake_reponse_packet: DSHandshakeResponsePacket, addr: SocketAddr) -> Result<(), String> {
        self.state.authenticate_daemon(addr, handshake_reponse_packet.challenge)?;

        info!("Authenticated");

        Ok(())
    }

    async fn handle_event(&self, event_packet: DSEventPacket, addr: SocketAddr) -> Result<(), String> {
        // debug!("Event: {:#?}", event_packet);

        self.state.send_event_from_daemon(&addr, event_packet.data).await
    }
}

#[async_trait]
impl Server for DaemonServer {
    fn get_tracing_name(&self) -> &'static str {
        "daemon"
    }

    fn get_bind_addr(&self) -> &'static str {
        &CONFIG.sockets.daemon
    }

    fn get_decrypter(&self) -> &'static RsaesJweDecrypter {
        &DECRYPTER
    }

    fn get_issuer(&self) -> &'static str {
        "aesterisk/daemon"
    }

    async fn on_accept(&self, addr: SocketAddr, tx: Tx) -> Result<(), String> {
        self.state.add_daemon(addr, tx);

        Ok(())
    }

    async fn on_disconnect(&self, addr: SocketAddr) -> Result<(), String> {
        self.state.remove_daemon(addr).await
    }

    async fn on_decrypt_error(&self, addr: SocketAddr) -> Result<(), String> {
        self.state.disconnect_daemon(addr)
    }

    #[instrument("daemon", skip(self, packet))]
    async fn on_packet(&self, packet: Packet, addr: SocketAddr) -> Result<(), String> {
        match packet.id {
            ID::DSAuth => {
                self.handle_auth(DSAuthPacket::parse(packet).expect("DSAuthPacket should be Some"), addr).await
            },
            ID::DSHandshakeResponse => {
                self.handle_handshake_response(DSHandshakeResponsePacket::parse(packet).expect("DSHandshakeResponsePacket should be Some"), addr).await
            }
            ID::DSEvent => {
                self.handle_event(DSEventPacket::parse(packet).expect("DSEventPacket should be Some"), addr).await
            },
            _ => {
                Err(format!("Should not receive [SW]* packet: {:?}", packet.id))
            },
        }
    }
}
