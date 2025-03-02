use std::sync::Arc;

use dashmap::DashMap;

use crate::types::{DaemonChannelMap, DaemonIDMap, DaemonKeyCache, DaemonListenMap, WebChannelMap, WebKeyCache, WebListenMap};

pub struct State {
    pub web_channel_map: WebChannelMap,
    pub web_key_cache: WebKeyCache,

    pub daemon_channel_map: DaemonChannelMap,
    pub daemon_key_cache: DaemonKeyCache,

    pub daemon_listen_map: DaemonListenMap,
    pub web_listen_map: WebListenMap,
    pub daemon_id_map: DaemonIDMap,
}

impl State {
    pub fn new() -> Self {
        Self {
            web_channel_map: Arc::new(DashMap::new()),
            web_key_cache: Arc::new(DashMap::new()),
            daemon_channel_map: Arc::new(DashMap::new()),
            daemon_key_cache: Arc::new(DashMap::new()),
            daemon_listen_map: Arc::new(DashMap::new()),
            web_listen_map: Arc::new(DashMap::new()),
            daemon_id_map: Arc::new(DashMap::new()),
        }
    }
}
