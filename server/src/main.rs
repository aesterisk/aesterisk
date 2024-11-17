use futures_util::join;
use josekit::jwk::alg::rsa::RsaKeyPair;
use lazy_static::lazy_static;

mod app;
mod config;
mod daemon;

lazy_static! {
    static ref CONFIG: config::Config = config::load_or_create("config.toml");
    static ref PRIVATE_KEY: josekit::jwk::Jwk = read_key(&CONFIG.server.private_key);
}

fn read_key(file: &str) -> josekit::jwk::Jwk {
    let pem = std::fs::read_to_string(file).expect("failed to read private key file");
    let key = RsaKeyPair::from_pem(pem).expect("failed to parse pem");
    key.to_jwk_private_key()
}

#[tokio::main]
async fn main() {
    let daemon_server_handle = tokio::spawn(daemon::start(&CONFIG, &PRIVATE_KEY));
    let app_server_handle = tokio::spawn(app::start(&CONFIG, &PRIVATE_KEY));

    join!(app_server_handle).0.expect("failed to join handle");
    join!(daemon_server_handle).0.expect("failed to join handle");
}
