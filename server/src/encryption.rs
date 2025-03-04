use std::time::{Duration, SystemTime};

use josekit::{jwe::{alg::rsaes::{RsaesJweDecrypter, RsaesJweEncrypter}, JweHeader}, jwk::alg::rsa::RsaKeyPair, jwt::{self, JwtPayload, JwtPayloadValidator}};
use lazy_static::lazy_static;

use packet::Packet;

use crate::config::CONFIG;

lazy_static! {
    pub static ref PRIVATE_KEY: josekit::jwk::Jwk = read_key(&CONFIG.server.private_key);
    pub static ref DECRYPTER: josekit::jwe::alg::rsaes::RsaesJweDecrypter = josekit::jwe::RSA_OAEP.decrypter_from_jwk(&PRIVATE_KEY).expect("decrypter should create successfully");
}

/// Reads an RSA private key from a PEM file and converts it to a JSON Web Key (JWK).
///
/// This function reads the entire content of the file at the specified path, expecting a PEM-encoded RSA private key. It then parses the PEM content into an RSA key pair and converts it into a JWK representing the private key.
///
/// # Panics
///
/// Panics if reading the file fails or if the PEM parsing is unsuccessful.
///
/// # Examples
///
/// ```
/// // Assuming "private_key.pem" exists and contains a valid RSA PEM-encoded key:
/// let jwk = read_key("private_key.pem");
/// // `jwk` can now be used for cryptographic operations.
/// ```
fn read_key(file: &str) -> josekit::jwk::Jwk {
    let pem = std::fs::read_to_string(file).expect("failed to read private key file");
    let key = RsaKeyPair::from_pem(pem).expect("failed to parse pem");
    key.to_jwk_private_key()
}

/// Encrypts a Packet into a JWT token using RSA encryption.
///
/// This function creates a JSON Web Encryption (JWE) header with predefined settings—namely, the "JWT" token type, RSA-OAEP algorithm,
/// and A256GCM content encryption. It constructs a JWT payload by embedding the packet data under the "p" claim, sets the issuer to
/// "aesterisk/server", records the current system time as the issued-at timestamp, and assigns an expiration 60 seconds later. The payload
/// is then encrypted using the provided encrypter.
///
/// If serialization of the packet, claim setting, or encryption fails, the function returns an error message encapsulated in a Result.
///
/// # Examples
///
/// ```
/// # use server::encryption::encrypt_packet;
/// # use server::packet::Packet;
/// # use josekit::jwe::RsaesJweEncrypter;
/// // Create a sample packet (adjust as needed based on the Packet implementation)
/// let packet = Packet::new();
///
/// // Initialize the encrypter (details depend on your RSA key configuration)
/// let encrypter = RsaesJweEncrypter::new("your-rsa-config").unwrap();
///
/// let token = encrypt_packet(packet, &encrypter).expect("Encryption failed");
/// println!("Encrypted token: {}", token);
/// ```pub fn encrypt_packet(packet: Packet, encrypter: &RsaesJweEncrypter) -> Result<String, String> {
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

/// Decrypts and validates a JWT token containing a packet.
///
/// This asynchronous function decrypts the provided JWT token using the given RSA JWE decrypter, then validates its payload by checking the issuer and issuance time constraints. On a successful validation, it extracts the packet from the "p" claim in the payload. If validation fails, the optional error callback is executed before returning an error.
///
/// # Panics
///
/// Panics if the token cannot be decrypted.
///
/// # Errors
///
/// Returns an error if the payload fails validation or if the packet cannot be parsed from the decrypted claims.
///
/// # Examples
///
/// ```rust
/// # use server::encryption::decrypt_packet;
/// # use server::packet::Packet;
/// # use josekit::jwe::RsaesJweDecrypter;
/// # async fn example(decrypter: &RsaesJweDecrypter) {
/// let encrypted_token = "eyJ..."; // A valid encrypted JWT token
/// let issuer = "my-issuer";
///
/// // Attempt to decrypt without an error callback.
/// let result: Result<Packet, String> = decrypt_packet(encrypted_token, decrypter, issuer, None).await;
///
/// match result {
///     Ok(packet) => {
///         // Process the packet
///     },
///     Err(err) => eprintln!("Decryption failed: {}", err),
/// }
/// # }
/// ```pub async fn decrypt_packet(msg: &str, decrypter: &RsaesJweDecrypter, issuer: &str, on_err: Option<impl AsyncFnOnce() -> Result<(), String>>) -> Result<Packet, String> {
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
