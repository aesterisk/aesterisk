use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

mod client;
mod status;

/// Starts the services and returns their join handles
pub fn start(token: CancellationToken) -> Vec<JoinHandle<Result<(), String>>> {
    vec![
        tokio::spawn(client::run(token.clone())),
        tokio::spawn(status::run(token)),
    ]
}
