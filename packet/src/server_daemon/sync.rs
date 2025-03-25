use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::{Packet, Version, ID};

// serde(rename = "...") is used to minimise data required to transfer sync packets

#[derive(Serialize, Deserialize, Debug)]
pub struct Network {
    #[serde(rename = "i")]
    pub id: u32,
    #[serde(rename = "s")]
    pub subnet: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Server {
    #[serde(rename = "i")]
    pub id: u32,
    #[serde(rename = "t")]
    pub tag: Tag,
    #[serde(rename = "e")]
    pub envs: Vec<Env>,
    #[serde(rename = "n")]
    pub networks: Vec<ServerNetwork>,
    #[serde(rename = "p")]
    pub ports: Vec<Port>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tag {
    #[serde(rename = "i")]
    pub image: String,
    #[serde(rename = "d")]
    pub docker_tag: String,
    #[serde(rename = "h")]
    pub healthcheck: Healthcheck,
    #[serde(rename = "m")]
    pub mounts: Vec<Mount>,
    #[serde(rename = "e")]
    pub env_defs: Vec<EnvDef>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Healthcheck {
    #[serde(rename = "t")]
    pub test: Vec<String>,
    #[serde(rename = "i")]
    pub interval: u64,
    #[serde(rename = "m")]
    pub timeout: u64,
    #[serde(rename = "r")]
    pub retries: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Mount {
    #[serde(rename = "c")]
    pub container_path: String,
    #[serde(rename = "h")]
    pub host_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EnvDef {
    #[serde(rename = "k")]
    pub key: String,
    #[serde(rename = "r")]
    pub required: bool,
    #[serde(rename = "t")]
    pub env_type: EnvType,
    #[serde(rename = "d")]
    pub default: Option<String>,
    #[serde(rename = "x")]
    pub regex: Option<String>,
    #[serde(rename = "m")]
    pub min: Option<i64>,
    #[serde(rename = "a")]
    pub max: Option<i64>,
    #[serde(rename = "i")]
    pub trim: bool,
}

#[derive(Serialize_repr, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum EnvType {
    Boolean = 0,
    Number = 1,
    String = 2,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Env {
    #[serde(rename = "k")]
    pub key: String,
    #[serde(rename = "v")]
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerNetwork {
    #[serde(rename = "n")]
    pub network: u32,
    #[serde(rename = "i")]
    pub ip: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Port {
    #[serde(rename = "p")]
    pub port: u16,
    #[serde(rename = "r")]
    pub protocol: Protocol,
    #[serde(rename = "m")]
    pub mapped: u16,
}

#[derive(Serialize_repr, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum Protocol {
    Tcp = 0,
    Udp = 1,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "Tcp"),
            Protocol::Udp => write!(f, "Udp"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SDSyncPacket {
    #[serde(rename = "n")]
    pub networks: Vec<Network>,
    #[serde(rename = "s")]
    pub servers: Vec<Server>,
}

impl SDSyncPacket {
    pub fn parse(packet: Packet) -> Option<Self> {
        if packet.id != ID::SDSync {
            return None;
        }

        match packet.version {
            Version::V0_1_0 => {
                let res = serde_json::from_value(packet.data);

                if res.is_err() {
                    println!("W (Packet) SDSync deserializing error: {:#?}", res.as_ref().expect_err("Result::err should return Some when Result::is_err returns true"));
                }

                res.ok()
            }
        }
    }

    pub fn to_packet(&self) -> Result<Packet, String> {
        let data = serde_json::to_value(self).map_err(|_| "packet data should be serializeable")?;
        Ok(Packet::new(Version::V0_1_0, ID::SDSync, data))
    }

    pub fn to_string(&self) -> Result<String, String> {
        let packet = self.to_packet()?;
        Ok(serde_json::to_string(&packet).map_err(|_| "Packet could not be serialized")?)
    }
}
