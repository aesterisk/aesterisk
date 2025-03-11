use std::{process, sync::Arc};

use clap::Parser;
use futures_channel::mpsc;
use futures_util::future::join_all;
use lazy_static::lazy_static;
use packet::events::EventType;
use tokio::{signal, sync::{Mutex, RwLock}};
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

mod config;
mod encryption;
mod logging;
mod packets;
mod services;

type Rx = mpsc::UnboundedReceiver<Message>;
type Tx = mpsc::UnboundedSender<Message>;

lazy_static! {
    static ref LISTENS: Arc<RwLock<Vec<EventType>>> = Arc::new(RwLock::new(Vec::new()));
    static ref SENDER: Arc<Mutex<Option<Tx>>> = Arc::new(Mutex::new(None));
}

#[repr(i32)]
enum ExitCode {
    Success = 0,
    ConfigError = 1,
    EncryptionError = 2,
    JoinError = 3,
    SignalError = 4,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> i32 {
        code as i32
    }
}

macro_rules! logo_str {
    () => {
r#"
           /$$$$$$                        /$$                         /$$           /$$      
  /$$/$$  /$$__  $$                      | $$                        |__/          | $$      
 |  $$$/ | $$  \ $$  /$$$$$$   /$$$$$$$ /$$$$$$    /$$$$$$   /$$$$$$  /$$  /$$$$$$$| $$   /$$
 /$$$$$$$| $$$$$$$$ /$$__  $$ /$$_____/|_  $$_/   /$$__  $$ /$$__  $$| $$ /$$_____/| $$  /$$/
|__ $$$_/| $$__  $$| $$$$$$$$|  $$$$$$   | $$    | $$$$$$$$| $$  \__/| $$|  $$$$$$ | $$$$$$/ 
  /$$ $$ | $$  | $$| $$_____/ \____  $$  | $$ /$$| $$_____/| $$      | $$ \____  $$| $$_  $$ 
 |__/__/ | $$  | $$|  $$$$$$$ /$$$$$$$/  |  $$$$/|  $$$$$$$| $$      | $$ /$$$$$$$/| $$ \  $$
         |__/  |__/ \_______/|_______/    \___/   \_______/|__/      |__/|_______/ |__/  \__/"#
    };
}

const AESTERISK_LOGO: &str = logo_str!();
const AESTERISK_LOGO_VERSION: &str = concat!(logo_str!(), "\n                                                                                    ");

/// Command line arguments
#[derive(Parser)]
#[command(version = concat!("v", env!("CARGO_PKG_VERSION")), name = AESTERISK_LOGO_VERSION, about = AESTERISK_LOGO, long_about = None)]
pub struct Cli {
    #[clap(short, long)]
    config: Option<String>,

    #[clap(short = 'u', long)]
    daemon_uuid: Option<String>,

    #[clap(short = 'k', long)]
    daemon_public_key: Option<String>,

    #[clap(short = 'p', long)]
    daemon_private_key: Option<String>,

    #[clap(short = 's', long)]
    server_url: Option<String>,

    #[clap(short = 'K', long)]
    server_public_key: Option<String>,

    #[clap(short = 'l', long)]
    logging_folder: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let mut exit_code = ExitCode::Success;

    logging::pre_init();

    let config = match config::init("config.toml", cli) {
        Ok(config) => config,
        Err(e) => {
            error!("Configuration error, please check your config file: {}", e);
            exit(ExitCode::ConfigError)
        }
    };

    logging::init();

    info!("Starting Aesterisk Daemon v{}", env!("CARGO_PKG_VERSION"));

    match encryption::init() {
        Ok(()) => (),
        Err(e) => {
            error!("Error initializing encryption: {}", e);
            exit(ExitCode::EncryptionError)
        }
    }

    if config.daemon.uuid.is_empty() {
        warn!("No Daemon ID set, please continue setup process!");
        exit(ExitCode::ConfigError)
    }

    if Uuid::parse_str(&config.daemon.uuid).is_err() {
        error!("Daemon ID is incorrectly set! Please check your config file.");
        exit(ExitCode::ConfigError)
    }

    let token = CancellationToken::new();

    let handles = services::start(token.clone());

    match signal::ctrl_c().await {
        Ok(()) => {
            warn!("Shutting down...");
        },
        Err(e) => {
            error!("Unable to listen for shutdown signal: {}", e);
            warn!("Shutting down...");
            exit_code = ExitCode::SignalError;
        }
    }

    token.cancel();

    info!("Waiting for services to stop...");

    let results = join_all(handles).await;

    if results.iter().any(|res| res.is_err()) {
        error!("Error joining handles");
        exit_code = ExitCode::JoinError;
    }

    exit(exit_code)
}

fn exit(code: ExitCode) -> ! {
    logging::flush();
    process::exit(code.into())
}
