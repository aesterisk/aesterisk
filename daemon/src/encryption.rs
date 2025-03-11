use std::{fs, sync::OnceLock, time::{Duration, SystemTime}};

use josekit::{jwe::{self, alg::rsaes::{RsaesJweDecrypter, RsaesJweEncrypter}, JweHeader}, jwk::alg::rsa::RsaKeyPair, jwt::{self, JwtPayload, JwtPayloadValidator}, Map, Value};
use packet::Packet;
use tracing::info;

use crate::config::{self, Config};

static DECRYPTER: OnceLock<RsaesJweDecrypter> = OnceLock::new();
static ENCRYPTER: OnceLock<RsaesJweEncrypter> = OnceLock::new();

fn decrypter() -> Result<&'static RsaesJweDecrypter, String> {
    DECRYPTER.get().ok_or("decrypter not initialized".to_string())
}

fn encrypter() -> Result<&'static RsaesJweEncrypter, String> {
    ENCRYPTER.get().ok_or("encrypter not initialized".to_string())
}

fn make_decrypter(config: &Config) -> Result<RsaesJweDecrypter, String> {
    match fs::read_to_string(&config.daemon.private_key) {
        Ok(pem) => {
            let decrypter = jwe::RSA_OAEP.decrypter_from_pem(pem.into_bytes()).map_err(|_| "Failed to parse PEM")?;
            info!("Loaded private RSA key from disk");
            Ok(decrypter)
        },
        Err(_) => {
            let key = RsaKeyPair::generate(2048).map_err(|_| "Failed to generate keys")?;
            fs::write(&config.daemon.private_key, key.to_pem_private_key()).map_err(|e| format!("Failed to save key to disk: {}", e))?;
            fs::write(&config.daemon.public_key, key.to_pem_public_key()).map_err(|e| format!("Failed to save key to disk: {}", e))?;
            info!("Generated RSA keys and saved to disk");
            Ok(jwe::RSA_OAEP.decrypter_from_pem(key.to_pem_private_key()).map_err(|_| "Failed to parse PEM")?)
        }
    }
}

fn make_encrypter(config: &Config) -> Result<RsaesJweEncrypter, String> {
    match fs::read_to_string(&config.server.public_key) {
        Ok(pem) => {
            let encrypter = jwe::RSA_OAEP.encrypter_from_pem(pem.into_bytes()).map_err(|_| "Failed to parse PEM")?;
            info!("Loaded public RSA key from disk");
            Ok(encrypter)
        },
        Err(_) => Err("Public key not specified".to_string())
    }
}

/// Encrypt a packet
pub fn encrypt_packet(packet: Packet) -> Result<String, String> {
    let mut header = JweHeader::new();
    header.set_token_type("JWT");
    header.set_algorithm("RSA-OAEP");
    header.set_content_encryption("A256GCM");

    let mut payload = JwtPayload::new();
    payload.set_claim("p", Some(serde_json::to_value(packet).map_err(|_| "Packet should be serializable")?)).map_err(|_| "Could not set payload claim")?;
    payload.set_issuer("aesterisk/daemon");
    payload.set_issued_at(&SystemTime::now());
    payload.set_expires_at(&SystemTime::now().checked_add(Duration::from_secs(60)).ok_or("Duration overflow")?);

    Ok(jwt::encode_with_encrypter(&payload, &header, encrypter()?).map_err(|_| "Could not encrypt packet")?)
}

/// Decrypt a packet
pub async fn decrypt_packet(msg: &str) -> Result<Packet, String> {
    let (payload, _) = jwt::decode_with_decrypter(msg, decrypter()?).map_err(|_| "Could not decrypt message")?;

    let mut validator = JwtPayloadValidator::new();
    validator.set_issuer("aesterisk/server");
    validator.set_base_time(SystemTime::now());
    validator.set_min_issued_time(SystemTime::now() - Duration::from_secs(60));
    validator.set_max_issued_time(SystemTime::now());

    match validator.validate(&payload) {
        Ok(()) => (),
        Err(e) => return Err(format!("Invalid token: {}", e)),
    }

    let payload: Map<String, Value> = payload.into();
    let try_packet = Packet::from_value(payload.into_iter().find_map(|(k, v)| if k == "p" { Some(v) } else { None }).ok_or("No payload found in packet")?);

    try_packet.ok_or(format!("Could not parse packet: \"{}\"", msg))
}

/// Initialize encryption.
///
/// Note: The configuration must be loaded before calling this function.
pub fn init() -> Result<(), String> {
    let config = config::get()?;

    if DECRYPTER.get().is_some() {
        return Err("decrypter already initialized".to_string());
    }

    if ENCRYPTER.get().is_some() {
        return Err("encrypter already initialized".to_string());
    }

    DECRYPTER.set(make_decrypter(config)?).map_err(|_| "decrypter was not set")?;
    ENCRYPTER.set(make_encrypter(config)?).map_err(|_| "encrypter was not set")?;

    Ok(())
}
