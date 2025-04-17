use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

mod client;
mod node_status;
mod server_status;

/// Starts the services and returns their join handles
pub fn start(token: CancellationToken) -> Vec<JoinHandle<Result<(), String>>> {
    vec![
        tokio::spawn(client::run(token.clone())),
        tokio::spawn(node_status::run(token.clone())),
        tokio::spawn(server_status::run(token)),
    ]
}
