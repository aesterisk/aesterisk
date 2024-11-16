use futures_util::join;

mod app;
mod daemon;

#[tokio::main]
async fn main() {
    let daemon_server_handle = tokio::spawn(daemon::start("0.0.0.0:31304"));
    let app_server_handle = tokio::spawn(app::start("0.0.0.0:31306"));

    join!(app_server_handle).0.expect("failed to join handle");
    join!(daemon_server_handle).0.expect("failed to join handle");
}
