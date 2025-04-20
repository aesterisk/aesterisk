use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Copy)]
pub enum EventType {
    NodeStatus,
    ServerStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeStatusEvent {
    pub online: bool,
    pub stats: Option<NodeStats>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeStats {
    pub used_memory: f64,
    pub total_memory: f64,
    pub cpu: f64,
    pub used_storage: f64,
    pub total_storage: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerStatusEvent {
    pub server: u32,
    pub status: ServerStatusType,
    pub memory: Option<Stats>,
    pub cpu: Option<Stats>,
    pub storage: Option<Stats>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatusType {
    /// Server is running (and healthy if healthcheck exists)
    Healthy,
    /// Server is starting 
    Starting,
    // Server is restarting
    Restarting,
    /// Server is stopping/removing
    Stopping,
    /// Server is not running
    Stopped,
    /// Server is running but is unhealthy
    Unhealthy,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Stats {
    pub used: f64,
    pub total: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EventData {
    NodeStatus(NodeStatusEvent),
    ServerStatus(ServerStatusEvent),
}

impl EventData {
    pub fn event_type(&self) -> EventType {
        match self {
            EventData::NodeStatus(_) => EventType::NodeStatus,
            EventData::ServerStatus(_) => EventType::ServerStatus,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub daemon: Uuid,
    pub event: EventData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListenEvent {
    pub event: EventType,
    pub daemons: Vec<Uuid>,
}
