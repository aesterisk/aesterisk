use std::time::{Duration, SystemTime};

use josekit::{jwe::{alg::rsaes::{RsaesJweDecrypter, RsaesJweEncrypter}, JweHeader}, jwk::alg::rsa::RsaKeyPair, jwt::{self, JwtPayload, JwtPayloadValidator}};
use lazy_static::lazy_static;

use packet::Packet;

use crate::config::CONFIG;

lazy_static! {
    pub static ref PRIVATE_KEY: josekit::jwk::Jwk = read_key(&CONFIG.server.private_key);
    pub static ref DECRYPTER: josekit::jwe::alg::rsaes::RsaesJweDecrypter = josekit::jwe::RSA_OAEP.decrypter_from_jwk(&PRIVATE_KEY).expect("decrypter should create successfully");
}

fn read_key(file: &str) -> josekit::jwk::Jwk {
    let pem = std::fs::read_to_string(file).expect("failed to read private key file");
    let key = RsaKeyPair::from_pem(pem).expect("failed to parse pem");
    key.to_jwk_private_key()
}

pub fn encrypt_packet(packet: Packet, encrypter: &RsaesJweEncrypter) -> Result<String, String> {
    let mut header = JweHeader::new();
    header.set_token_type("JWT");
    header.set_algorithm("RSA-OAEP");
    header.set_content_encryption("A256GCM");

    let mut payload = JwtPayload::new();
    payload.set_claim("p", Some(serde_json::to_value(packet).map_err(|_| "packet should be serializable")?)).map_err(|_| "should set claim correctly")?;
    payload.set_issuer("aesterisk/server");
    payload.set_issued_at(&SystemTime::now());
    payload.set_expires_at(&SystemTime::now().checked_add(Duration::from_secs(60)).ok_or("duration overflow")?);

    Ok(jwt::encode_with_encrypter(&payload, &header, encrypter).map_err(|_| "could not encrypt token")?)
}

pub async fn decrypt_packet(msg: &str, decrypter: &RsaesJweDecrypter, issuer: &str, on_err: Option<impl AsyncFnOnce() -> Result<(), String>>) -> Result<Packet, String> {
    let (payload, _) = jwt::decode_with_decrypter(msg, decrypter).expect("should decrypt");

    let mut validator = JwtPayloadValidator::new();
    validator.set_issuer(issuer);
    validator.set_base_time(SystemTime::now());
    validator.set_min_issued_time(SystemTime::now() - Duration::from_secs(60));
    validator.set_max_issued_time(SystemTime::now());

    match validator.validate(&payload) {
        Ok(()) => (),
        Err(e) => {
            if on_err.is_some() {
                on_err.unwrap()().await?;
            }

            return Err(format!("Invalid token: {}", e));
        }
    }

    // TODO: maybe don't clone hehe
    let try_packet = Packet::from_value(payload.claim("p").expect("should have .p").clone());

    try_packet.ok_or(format!("Could not parse packet: \"{}\"", msg))
}
