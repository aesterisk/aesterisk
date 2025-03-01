use std::sync::Arc;

use daemon_server::DaemonServer;
use futures_util::join;
use server::Server;
use tracing::{info, warn};

mod config;
mod daemon;
mod daemon_server;
mod db;
mod logging;
mod server;
mod statics;
mod types;
mod web;

#[dotenvy::load]
#[tokio::main]
async fn main() {
    logging::init();

    info!("Starting Aesterisk Server v{}", env!("CARGO_PKG_VERSION"));

    db::init().await.expect("failed to initialize database connection");

    let daemon_server = Arc::new(DaemonServer::new());

    info!("Starting Daemon Server...");
    let daemon_server_handle = tokio::spawn(daemon_server.start());

    info!("Starting Web Server...");
    let web_server_handle = tokio::spawn(web::start());

    let (web_res, daemon_res) = join!(web_server_handle, daemon_server_handle);
    web_res.expect("failed to join web server handle");
    daemon_res.expect("failed to join daemon server handle");

    warn!("Internal servers are down, exiting...");

    // TODO: as this is the main server, and exit should probably immediately notify us, but as
    //       this is a prototype, we'll just let it exit for now. as something might have failed,
    //       we can't rely on the notification being sent, so we'll need to monitor the server
    //       status from the outside as well.
}
