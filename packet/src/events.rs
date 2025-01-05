use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Copy)]
pub enum EventType {
    NodeStatus,
    OtherEvent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeStatusEvent {
    pub online: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OtherEvent {
    pub num: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EventData {
    NodeStatus(NodeStatusEvent),
    OtherEvent(OtherEvent),
}

impl EventData {
    pub fn event_type(&self) -> EventType {
        match self {
            EventData::NodeStatus(_) => EventType::NodeStatus,
            EventData::OtherEvent(_) => EventType::OtherEvent,
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
