use std::{env, io};

use futures_util::join;
use josekit::jwk::alg::rsa::RsaKeyPair;
use lazy_static::lazy_static;
use sqlx::postgres::PgPoolOptions;
use tracing::{info, warn, Level};
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod daemon;
mod web;

lazy_static! {
    static ref CONFIG: config::Config = config::load_or_create("config.toml");
    static ref PRIVATE_KEY: josekit::jwk::Jwk = read_key(&CONFIG.server.private_key);
    static ref DECRYPTER: josekit::jwe::alg::rsaes::RsaesJweDecrypter = josekit::jwe::RSA_OAEP.decrypter_from_jwk(&PRIVATE_KEY).expect("decrypter should create successfully");
}

fn read_key(file: &str) -> josekit::jwk::Jwk {
    let pem = std::fs::read_to_string(file).expect("failed to read private key file");
    let key = RsaKeyPair::from_pem(pem).expect("failed to parse pem");
    key.to_jwk_private_key()
}

#[dotenvy::load]
#[tokio::main]
async fn main() {
    let logs_rotation = tracing_appender::rolling::Builder::new().filename_suffix("server.aesterisk.log").rotation(Rotation::DAILY).build(&CONFIG.logging.folder).expect("could not initialize file logger");
    let (logs_file, _logs_file_guard) = tracing_appender::non_blocking(logs_rotation);
    let logs_file_layer = tracing_subscriber::fmt::layer().with_writer(logs_file.with_max_level(Level::DEBUG)).with_ansi(false);

    let (logs_stdout, _logs_stdout_guard) = tracing_appender::non_blocking(io::stdout());
    let (logs_stderr, _logs_stderr_guard) = tracing_appender::non_blocking(io::stderr());
    let logs_stdout_layer = tracing_subscriber::fmt::layer().with_writer(logs_stderr.with_max_level(Level::WARN).or_else(logs_stdout.with_max_level(Level::DEBUG))).with_ansi(true);

    tracing_subscriber::registry().with(logs_file_layer).with(logs_stdout_layer).init();

    info!("Starting Aesterisk Server v{}", env!("CARGO_PKG_VERSION"));

    let pool = PgPoolOptions::new().max_connections(5).connect(&env::var("DATABASE_URL").expect("environment variable DATABASE_URL needs to be set")).await.expect("could not connect to database");

    info!("Starting Daemon Server...");
    let daemon_server_handle = tokio::spawn(daemon::start(pool.clone()));
    info!("Starting Web Server...");
    let web_server_handle = tokio::spawn(web::start(pool.clone()));

    let (web_res, daemon_res) = join!(web_server_handle, daemon_server_handle);
    web_res.expect("failed to join handle");
    daemon_res.expect("failed to join handle");

    warn!("Web and Daemon Servers are down, exiting...");
}
