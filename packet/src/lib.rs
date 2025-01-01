pub mod app_server;
pub mod server_app;
pub mod daemon_server;
pub mod server_daemon;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Packet {
    pub version: Version,
    pub id: ID,
    pub data: serde_json::Value,
}

#[derive(serde_repr::Serialize_repr, serde_repr::Deserialize_repr, Debug, PartialEq)]
#[repr(u8)]
pub enum Version {
    V0_1_0 = 0,
}

#[derive(serde_repr::Serialize_repr, serde_repr::Deserialize_repr, Debug, PartialEq)]
#[repr(u8)]
pub enum ID {
    ASAuth = 0,
    DSAuth = 1,
    SAHandshakeRequest = 2,
    SDHandshakeRequest = 3,
    ASHandshakeResponse = 4,
    DSHandshakeResponse = 5,
    SAAuthResponse = 6,
    SDAuthResponse = 7,
    ASListen = 8,
    SDListen = 9,
    DSEvent = 10,
    SAEvent = 11,
}

impl Packet {
    pub fn new(version: Version, id: ID, data: serde_json::Value) -> Self {
        Self {
            version,
            id,
            data,
        }
    }

    pub fn from_str(msg: &str) -> Option<Self> {
        let res = serde_json::from_str(msg);

        if res.is_err() {
            println!("W (Packet) Packet deserializing error: {:#?}", res.as_ref().err().expect("Result::err should return Some when Result::is_err returns true"));
        }

        res.ok()
    }

    pub fn from_value(value: serde_json::Value) -> Option<Self> {
        let res = serde_json::from_value(value);

        if res.is_err() {
            println!("W (Packet) Packet deserializing error: {:#?}", res.as_ref().err().expect("Result::err should return Some when Result::is_err returns true"));
        }

        res.ok()
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(&self).expect("failed to serialize packet")
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct NodeStatus {
    pub id: u32,
    pub status: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum Event {
    NodesList(Vec<NodeStatus>),
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum ListenEvent {
    NodesList(Vec<u32>),
}
