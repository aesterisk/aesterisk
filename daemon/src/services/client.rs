use std::time::Duration;

use futures_channel::mpsc::unbounded;
use futures_util::{future, pin_mut, FutureExt, StreamExt, TryStreamExt};
use packet::daemon_server::auth::DSAuthPacket;
use tokio::select;
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{config, encryption, packets, Rx, LISTENS, SENDER};

/// Runs the client service, connecting to the Aesterisk Server
pub async fn run(token: CancellationToken) -> Result<(), String> {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut attempts = 0;

    loop {
        if attempts <= 5 || attempts % 1800 == 0 {
            info!("Connecting to server...");
        }

        let (tx, rx) = unbounded();
        SENDER.lock().await.replace(tx);

        *LISTENS.write().await = Vec::new();
        select!(
            res = tokio::spawn(connect_to_server(rx)) => {
                match res {
                    Ok(Ok(())) => {
                        attempts = 1;
                    },
                    Ok(Err(e)) => if attempts <= 5 || attempts % 1800 == 0 {
                        error!("{}", e);
                    },
                    Err(_) => if attempts <= 5 || attempts % 1800 == 0 {
                        error!("Couldn't join connection handle");
                    },
                }

                attempts += 1;
                
                // TODO: Implement exponential backoff
                // TODO: maybe add a limit to the amount of attempts
                // TODO: don't hardcode logging attempts
                if attempts <= 5 || attempts % 1800 == 0 {
                    warn!("Disconnected from server, retrying... (attempt {})", attempts);
                } else if attempts == 6 {
                    warn!("Max logged attempts reached, further attempts will be logged every 30 minutes (retrying in the background otherwise)"); // cuz 1800 secs = 30 min
                }

                interval.tick().await;
            },
            _ = token.cancelled() => {
                warn!("Disconnecting from server");

                if let Some(sender) = SENDER.lock().await.take() {
                    sender.close_channel();
                }

                break;
            }
        );
    }

    Ok(())
}

// TODO: move to a common crate for use in both the server and the daemon
fn error_to_string(e: tungstenite::Error) -> String {
    match e {
        tungstenite::Error::Utf8 => "Error in UTF-8 encoding".to_string(),
        tungstenite::Error::Io(e) => format!("IO error ({})", e.kind()),
        tungstenite::Error::Tls(_) => "TLS error".to_string(),
        tungstenite::Error::Url(_) => "Invalid URL".to_string(),
        tungstenite::Error::Http(_) => "HTTP error".to_string(),
        tungstenite::Error::HttpFormat(_) => "HTTP format error".to_string(),
        tungstenite::Error::Capacity(_) => "Buffer capacity exhausted".to_string(),
        tungstenite::Error::Protocol(_) => "Protocol violation".to_string(),
        tungstenite::Error::AlreadyClosed => "Connection already closed".to_string(),
        tungstenite::Error::AttackAttempt => "Attack attempt detected".to_string(),
        tungstenite::Error::WriteBufferFull(_) => "Write buffer full".to_string(),
        tungstenite::Error::ConnectionClosed => "Connection closed".to_string(),
    }
}

async fn connect_to_server(rx: Rx) -> Result<(), String> {
    let config = config::get()?;

    let (stream, _) = tokio_tungstenite::connect_async(&config.server.url).await.map_err(|e| format!("Could not connect to server: {}", error_to_string(e)))?;

    info!("Connected to server");
    let (write, read) = stream.split();

    info!("Authenticating...");
    tokio::spawn(handle_connection().then(|res| match res {
        Ok(()) => future::ready(()),
        Err(e) => {
            error!("Error authenticating: {}", e);
            future::ready(())
        }
    }));

    let incoming = read.try_filter(|msg| future::ready(msg.is_text())).for_each(|msg| async {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                error!("{}", error_to_string(e));
                return;
            }
        };

        let text = match msg.into_text() {
            Ok(text) => text,
            Err(e) => {
                error!("{}", error_to_string(e));
                return;
            }
        };

        tokio::spawn(packets::handle(text).then(|res| match res {
            Ok(()) => future::ready(()),
            Err(e) => {
                error!("Error handling packet: {}", e);
                future::ready(())
            }
        }));
    });

    let outgoing = rx.map(Ok).forward(write);

    pin_mut!(incoming, outgoing);
    future::select(incoming, outgoing).await;

    Ok(())
}

async fn handle_connection() -> Result<(), String> {
    let config = config::get()?;

    SENDER.lock().await.as_ref().ok_or("sender is not available")?.unbounded_send(
        Message::Text(
            encryption::encrypt_packet(
                DSAuthPacket {
                    daemon_uuid: config.daemon.uuid.clone()
                }.to_packet()?,
            )?
        )
    ).map_err(|e| format!("Could not send message: {}", e))?;

    Ok(())
}
