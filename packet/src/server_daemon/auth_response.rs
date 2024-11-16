use crate::{Packet, Version, ID};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct SDAuthResponsePacket {
    pub success: bool,
}

impl SDAuthResponsePacket {
    pub fn parse(packet: Packet) -> Option<Self> {
        if packet.id != ID::SDAuthResponse {
            return None;
        }

        match packet.version {
            Version::V0_1_0 => {
                let res = serde_json::from_value(packet.data);

                if res.is_err() {
                    println!("W (Packet) SDAuthResponsePacket deserializing error: {:#?}", res.as_ref().err().expect("Result::err should return Some when Result::is_err returns true"));
                }

                res.ok()
            }
        }
    }

    pub fn to_string(&self) -> Result<String, serde_json::Error> {
        let data = serde_json::to_value(&self).expect("packet data should be serializeable");
        let packet = Packet::new(Version::V0_1_0, ID::SDAuthResponse, data);
        serde_json::to_string(&packet)
    }
}
