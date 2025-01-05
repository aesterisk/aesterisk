use crate::{events::EventData, Packet, Version, ID};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct DSEventPacket {
    pub data: EventData,
}

impl DSEventPacket {
    pub fn parse(packet: Packet) -> Option<Self> {
        if packet.id != ID::DSEvent {
            return None;
        }

        match packet.version {
            Version::V0_1_0 => {
                let res = serde_json::from_value(packet.data);

                if res.is_err() {
                    println!("W (Packet) DSEvent deserializing error: {:#?}", res.as_ref().err().expect("Result::err should return Some when Result::is_err returns true"));
                }

                res.ok()
            }
        }
    }

    pub fn to_packet(&self) -> Result<Packet, String> {
        let data = serde_json::to_value(&self).map_err(|_| "packet data should be serializeable")?;
        Ok(Packet::new(Version::V0_1_0, ID::DSEvent, data))
    }

    pub fn to_string(&self) -> Result<String, String> {
        let packet = self.to_packet()?;
        Ok(serde_json::to_string(&packet).map_err(|_| "Packet could not be serialized")?)
    }
}
