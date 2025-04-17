use bollard::secret::ContainerSummary;
use futures_util::StreamExt;
use packet::{
    daemon_server::event::DSEventPacket,
    events::{EventData, EventType, ServerStatus, ServerStatusEvent, ServerStatusType, Stats},
};
use tokio::select;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::{LISTENS, SENDER, docker, encryption};

/// Runs the server status service, sending status information to the clients
pub async fn run(token: CancellationToken) -> Result<(), String> {
    select! {
        _ = token.cancelled() => {
            warn!("Stopping server status service");
            Ok(())
        },
        res = send_loop() => {
            res
        }
    }
}

fn server_status(server: &ContainerSummary) -> Result<ServerStatusType, String> {
    let state = server.state.as_ref().ok_or("no state")?;

    Ok(match state.to_lowercase().as_ref() {
        "running" => ServerStatusType::Healthy,
        "created" => ServerStatusType::Starting,
        "exited" => ServerStatusType::Offline,
        "restarting" => ServerStatusType::Starting,
        "stopping" => ServerStatusType::Stopping,
        _ => unimplemented!("unknown state: {}", state),
    })
}

async fn send_loop() -> Result<(), String> {
    /*// TODO: make this configurable
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    const GB: f64 = 1_073_741_824.0;

    loop {
        interval.tick().await;

        if !LISTENS.read().await.contains(&EventType::ServerStatus) {
            continue;
        }

        // TODO: this is NOT efficient AT ALL! PLEASE fix this ASAP!
        //       (it'll prob take months before I get to this)

        let servers = docker::server::get_servers().await?;

        let mut all_stats = vec![];

        for server in servers {
            let id = server.labels.as_ref().ok_or("no labels found")?.get("io.aesterisk.server.id").ok_or("no id found")?.parse().map_err(|_| "could not parse id")?;

            let stats = docker::get()?.stats("", Some(StatsOptions {
                stream: false,
                one_shot: false,
            })).next().await;

            let status = server_status(&server)?;

            match stats {
                None => debug!("offline (prob)"),
                Some(Ok(s)) => all_stats.push(ServerStatus {
                    server: id,
                    cpu: match status {
                        ServerStatusType::Healthy | ServerStatusType::Starting | ServerStatusType::Stopping => Some(Stats {
                            used: s.cpu_stats.cpu_usage.total_usage as f64,
                            total: s.cpu_stats.online_cpus.ok_or("no cpu_stats.online_cpus")? as f64,
                        }),
                        _ => None,
                    },
                    memory: match status {
                        ServerStatusType::Healthy | ServerStatusType::Starting | ServerStatusType::Stopping => Some(Stats {
                            used: s.memory_stats.usage.ok_or("no memory_stats.usage")? as f64 / GB,
                            total: s.memory_stats.limit.ok_or("no memory_stats.usage")? as f64 / GB,
                        }),
                        _ => None,
                    },
                    storage: Some(Stats {
                        used: server.size_root_fs.ok_or("no size_root_fs")? as f64 / GB,
                        total: 100.0,
                    }),
                    status,
                }),
                Some(Err(e)) => error!("error getting stats: {}", e),
            }

            debug!("if this is before, it works a single time...");
        }

        if SENDER.lock().await.is_some() {
            let packet = DSEventPacket {
                data: EventData::ServerStatus(ServerStatusEvent {
                    statuses: all_stats,
                }),
            };

            let packet = match packet.to_packet() {
                Ok(packet) => packet,
                Err(e) => {
                    error!("Error creating packet: {}", e);
                    continue;
                }
            };

            let packet = match encryption::encrypt_packet(packet) {
                Ok(packet) => packet,
                Err(e) => {
                    error!("Error encrypting packet: {}", e);
                    continue;
                }
            };

            if let Some(tx) = SENDER.lock().await.as_ref() {
                match tx.unbounded_send(Message::Text(packet)) {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Could not send packet: {}", e);
                        continue;
                    }
                }
            }
        }
    }*/
    Ok(())
}
