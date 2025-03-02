use std::sync::Arc;

use dashmap::DashMap;
use josekit::jwk::alg::rsa::RsaKeyPair;
use lazy_static::lazy_static;

use crate::{config, types::{DaemonChannelMap, DaemonIDMap, DaemonKeyCache, DaemonListenMap, WebChannelMap, WebKeyCache, WebListenMap}};

lazy_static! {
    pub static ref CONFIG: config::Config = config::load_or_create("config.toml");
    pub static ref PRIVATE_KEY: josekit::jwk::Jwk = read_key(&CONFIG.server.private_key);
    pub static ref DECRYPTER: josekit::jwe::alg::rsaes::RsaesJweDecrypter = josekit::jwe::RSA_OAEP.decrypter_from_jwk(&PRIVATE_KEY).expect("decrypter should create successfully");

//    pub static ref WEB_CHANNEL_MAP: WebChannelMap = Arc::new(DashMap::new());
//    pub static ref WEB_KEY_CACHE: WebKeyCache = Arc::new(DashMap::new());
//
//    pub static ref DAEMON_CHANNEL_MAP: DaemonChannelMap = Arc::new(DashMap::new());
//    pub static ref DAEMON_KEY_CACHE: DaemonKeyCache = Arc::new(DashMap::new());
//
//    pub static ref DAEMON_LISTEN_MAP: DaemonListenMap = Arc::new(DashMap::new());
//    pub static ref WEB_LISTEN_MAP: WebListenMap = Arc::new(DashMap::new());
//    pub static ref DAEMON_ID_MAP: DaemonIDMap = Arc::new(DashMap::new());
}

fn read_key(file: &str) -> josekit::jwk::Jwk {
    let pem = std::fs::read_to_string(file).expect("failed to read private key file");
    let key = RsaKeyPair::from_pem(pem).expect("failed to parse pem");
    key.to_jwk_private_key()
}
