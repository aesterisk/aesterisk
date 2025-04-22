use std::sync::OnceLock;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

mod client;
mod node_status;
pub mod server_status;

static CANCELLATION_TOKEN: OnceLock<CancellationToken> = OnceLock::new();

pub fn get_cancellation_token() -> Option<CancellationToken> {
    CANCELLATION_TOKEN.get().cloned()
}

/// Starts the services and returns their join handles.
/// Should only be called **once**.
pub fn start(token: CancellationToken) -> Result<Vec<JoinHandle<Result<(), String>>>, String> {
    CANCELLATION_TOKEN.set(token).map_err(|_| "cancellation token already set")?;

    Ok(vec![
        tokio::spawn(client::run(get_cancellation_token().ok_or("cancellation token should already be set")?)),
        tokio::spawn(node_status::run(get_cancellation_token().ok_or("cancellation token should already be set")?)),
    ])
}
