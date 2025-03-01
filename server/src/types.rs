use std::{collections::{HashMap, HashSet}, net::SocketAddr, sync::Arc};

use dashmap::DashMap;
use futures_channel::mpsc;
use packet::events::EventType;
use sqlx::types::Uuid;
use tokio_tungstenite::tungstenite::Message;

use crate::{daemon::DaemonSocket, web::WebSocket};

pub type Tx = mpsc::UnboundedSender<Message>;
pub type Rx = mpsc::UnboundedReceiver<Message>;

pub type WebChannelMap = Arc<DashMap<SocketAddr, WebSocket>>;
pub type WebKeyCache = Arc<DashMap<u32, Arc<Vec<u8>>>>;

pub type DaemonChannelMap = Arc<DashMap<SocketAddr, DaemonSocket>>;
pub type DaemonKeyCache = Arc<DashMap<Uuid, Arc<Vec<u8>>>>;

pub type DaemonListenMap = Arc<DashMap<Uuid, HashMap<EventType, HashSet<SocketAddr>>>>;
pub type WebListenMap = Arc<DashMap<SocketAddr, HashMap<EventType, HashSet<Uuid>>>>;
pub type DaemonIDMap = Arc<DashMap<Uuid, SocketAddr>>;
