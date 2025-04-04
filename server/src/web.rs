use std::{borrow::Borrow, net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use packet::{web_server::{auth::WSAuthPacket, handshake_response::WSHandshakeResponsePacket, listen::WSListenPacket, sync::WSSyncPacket}, Packet, ID};
use tracing::{debug, info, instrument};

use crate::{config::CONFIG, db, encryption::DECRYPTER, server::Server, state::{State, Tx, WebKeyCache}};

/// WebServer is a WebSocket server (implemented by the `Server` trait) that listens for web
/// (frontend) connections.
pub struct WebServer {
    state: Arc<State>,
}

struct PublicKeyQuery {
    user_public_key: String,
}

impl WebServer {
    /// Creates a new `WebServer` instance, with the given `State`.
    pub fn new(state: Arc<State>) -> Self {
        Self {
            state
        }
    }

    async fn query_user_public_key(&self, user_id: u32) -> Result<Arc<Vec<u8>>, String> {
        {
            let cache: &WebKeyCache = self.state.web_key_cache.borrow();
            if let Some(v) = cache.get(&user_id) {
                return Ok(v.clone());
            }
        }

        let res = sqlx::query_as!(PublicKeyQuery, "SELECT user_public_key FROM aesterisk.users WHERE user_id = $1", user_id as i32).fetch_one(db::get()?).await.map_err(|_| format!("User with ID {} does not exist", user_id))?;

        let cache: &WebKeyCache = self.state.web_key_cache.borrow();
        cache.insert(user_id, Arc::new(res.user_public_key.into_bytes()));
        Ok(cache.get(&user_id).ok_or("key should be in cache")?.clone())
    }

    async fn handle_auth(&self, auth_packet: WSAuthPacket, addr: SocketAddr) -> Result<(), String> {
        let key = self.query_user_public_key(auth_packet.user_id).await?;

        self.state.send_web_handshake_request(&addr, auth_packet.user_id, key)
    }

    async fn handle_handshake_response(&self, handshake_reponse_packet: WSHandshakeResponsePacket, addr: SocketAddr) -> Result<(), String> {
        self.state.authenticate_web(addr, handshake_reponse_packet.challenge)?;

        info!("Authenticated");

        Ok(())
    }

    async fn handle_listen(&self, listen_packet: WSListenPacket, addr: SocketAddr) -> Result<(), String> {
        // debug!("Handling listen packet: {:#?}", listen_packet);

        self.state.send_listen(addr, listen_packet.events).await
    }

    async fn handle_sync(&self, sync_packet: WSSyncPacket) -> Result<(), String> {
        debug!("Handling sync packet: {:#?}", sync_packet);

        self.state.sync_daemon(sync_packet.daemon, None).await
    }
}

#[async_trait]
impl Server for WebServer {
    fn get_bind_addr(&self) ->  &'static str {
        &CONFIG.sockets.web
    }

    fn get_tracing_name(&self) -> &'static str {
        "web"
    }

    fn get_issuer(&self) ->  &'static str {
        "aesterisk/web"
    }

    fn get_decrypter(&self) ->  &'static josekit::jwe::alg::rsaes::RsaesJweDecrypter {
        &DECRYPTER
    }

    async fn on_accept(&self, addr: SocketAddr, tx: Tx) -> Result<(), String> {
        self.state.add_web(addr, tx);

        Ok(())
    }

    async fn on_disconnect(&self, addr: SocketAddr) -> Result<(), String> {
        self.state.remove_web(addr).await
    }

    async fn on_decrypt_error(&self, addr: SocketAddr) -> Result<(), String> {
        self.state.disconnect_web(addr)
    }

    #[instrument("web", skip(self, packet))]
    async fn on_packet(&self, packet: Packet, addr: SocketAddr) -> Result<(), String> {
        match packet.id {
            ID::WSAuth => {
                self.handle_auth(WSAuthPacket::parse(packet).ok_or("Could not parse WSAuthPacket")?, addr).await
            },
            ID::WSHandshakeResponse => {
                self.handle_handshake_response(WSHandshakeResponsePacket::parse(packet).ok_or("Could not parse WSHandshakeResponsePacket")?, addr).await
            }
            ID::WSListen => {
                self.handle_listen(WSListenPacket::parse(packet).ok_or("Could not parse WSListenPacket")?, addr).await
            },
            ID::WSSync => {
                self.handle_sync(WSSyncPacket::parse(packet).ok_or("Could not parse WSSyncPacket")?).await
            }
            _ => {
                Err(format!("Should not receive [SD]* packet: {:?}", packet.id))
            },
        }
    }
}
