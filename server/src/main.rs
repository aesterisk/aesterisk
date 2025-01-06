use std::{collections::{HashMap, HashSet}, env, io, net::SocketAddr, sync::Arc};

use daemon::DaemonSocket;
use futures_channel::mpsc;
use futures_util::join;
use josekit::jwk::alg::rsa::RsaKeyPair;
use lazy_static::lazy_static;
use packet::events::EventType;
use sqlx::{postgres::PgPoolOptions, types::Uuid};
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn, Level};
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt};
use web::WebSocket;

mod config;
mod daemon;
mod web;

type Tx = mpsc::UnboundedSender<Message>;
type Rx = mpsc::UnboundedReceiver<Message>;

type WebChannelMap = Arc<RwLock<HashMap<SocketAddr, WebSocket>>>;
type WebKeyCache = Arc<Mutex<HashMap<u32, Arc<Vec<u8>>>>>;

type DaemonChannelMap = Arc<RwLock<HashMap<SocketAddr, DaemonSocket>>>;
type DaemonKeyCache = Arc<Mutex<HashMap<Uuid, Arc<Vec<u8>>>>>;

type DaemonListenMap = Arc<RwLock<HashMap<Uuid, HashMap<EventType, HashSet<SocketAddr>>>>>;
type WebListenMap = Arc<RwLock<HashMap<SocketAddr, HashMap<EventType, HashSet<Uuid>>>>>;
type DaemonIDMap = Arc<RwLock<HashMap<Uuid, SocketAddr>>>;

lazy_static! {
    static ref CONFIG: config::Config = config::load_or_create("config.toml");
    static ref PRIVATE_KEY: josekit::jwk::Jwk = read_key(&CONFIG.server.private_key);
    static ref DECRYPTER: josekit::jwe::alg::rsaes::RsaesJweDecrypter = josekit::jwe::RSA_OAEP.decrypter_from_jwk(&PRIVATE_KEY).expect("decrypter should create successfully");

    static ref WEB_CHANNEL_MAP: WebChannelMap = Arc::new(RwLock::new(HashMap::new()));
    static ref WEB_KEY_CACHE: WebKeyCache = Arc::new(Mutex::new(HashMap::new()));

    static ref DAEMON_CHANNEL_MAP: DaemonChannelMap = Arc::new(RwLock::new(HashMap::new()));
    static ref DAEMON_KEY_CACHE: DaemonKeyCache = Arc::new(Mutex::new(HashMap::new()));

    static ref DAEMON_LISTEN_MAP: DaemonListenMap = Arc::new(RwLock::new(HashMap::new()));
    static ref WEB_LISTEN_MAP: WebListenMap = Arc::new(RwLock::new(HashMap::new()));
    static ref DAEMON_ID_MAP: DaemonIDMap = Arc::new(RwLock::new(HashMap::new()));
}

fn read_key(file: &str) -> josekit::jwk::Jwk {
    let pem = std::fs::read_to_string(file).expect("failed to read private key file");
    let key = RsaKeyPair::from_pem(pem).expect("failed to parse pem");
    key.to_jwk_private_key()
}

#[dotenvy::load]
#[tokio::main]
async fn main() {
    #[cfg(feature = "tokio_debug")]
    let console_layer = console_subscriber::Builder::default().spawn();

    let logs_rotation = tracing_appender::rolling::Builder::new().filename_suffix("server.aesterisk.log").rotation(Rotation::DAILY).build(&CONFIG.logging.folder).expect("could not initialize file logger");
    let (logs_file, _logs_file_guard) = tracing_appender::non_blocking(logs_rotation);
    let logs_file_layer = tracing_subscriber::fmt::layer().with_writer(logs_file.with_max_level(Level::DEBUG)).with_ansi(false);

    let (logs_stdout, _logs_stdout_guard) = tracing_appender::non_blocking(io::stdout());
    let (logs_stderr, _logs_stderr_guard) = tracing_appender::non_blocking(io::stderr());
    let logs_stdout_layer = tracing_subscriber::fmt::layer().with_writer(logs_stderr.with_max_level(Level::WARN).or_else(logs_stdout.with_max_level(Level::DEBUG))).with_ansi(true);

    #[cfg(feature = "tokio_debug")]
    tracing_subscriber::registry()
        .with(console_layer)
        .with(logs_file_layer)
        .with(logs_stdout_layer)
        .init();

    #[cfg(not(feature = "tokio_debug"))]
    tracing_subscriber::registry()
        .with(logs_file_layer)
        .with(logs_stdout_layer)
        .init();

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
