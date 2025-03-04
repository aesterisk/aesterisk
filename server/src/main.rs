use std::sync::Arc;

use futures_util::join;
use state::State;
use tracing::{info, warn};

use daemon::DaemonServer;
use web::WebServer;
use server::Server;

mod config;
mod daemon;
mod db;
mod encryption;
mod logging;
mod server;
mod state;
mod web;

#[dotenvy::load]
#[tokio::main]
/// Asynchronous entry point for the Aesterisk Server application.
/// 
/// This function initializes logging, establishes a database connection,
/// and sets up a shared application state. It then concurrently starts the
/// Daemon and Web servers as asynchronous tasks and awaits their completion.
/// Panics are triggered if the database initialization fails or if either
/// server task does not join successfully.
/// 
/// # Examples
/// 
/// To start the Aesterisk Server, compile and run the binary:
/// 
/// ```bash
/// cargo run
/// ```
async fn main() {
    logging::init();

    info!("Starting Aesterisk Server v{}", env!("CARGO_PKG_VERSION"));

    db::init().await.expect("failed to initialize database connection");

    let state = Arc::new(State::new());

    let daemon_server = Arc::new(DaemonServer::new(Arc::clone(&state)));
    let web_server = Arc::new(WebServer::new(Arc::clone(&state)));

    info!("Starting Daemon Server...");
    let daemon_server_handle = tokio::spawn(daemon_server.start());

    info!("Starting Web Server...");
    let web_server_handle = tokio::spawn(web_server.start());

    let (web_res, daemon_res) = join!(web_server_handle, daemon_server_handle);
    web_res.expect("failed to join web server handle");
    daemon_res.expect("failed to join daemon server handle");

    warn!("Internal servers are down, exiting...");

    // TODO: as this is the main server, and exit should probably immediately notify us, but as
    //       this is a prototype, we'll just let it exit for now. as something might have failed,
    //       we can't rely on the notification being sent, so we'll need to monitor the server
    //       status from the outside as well.
}
