use std::sync::Arc;

use bollard::{container::{InspectContainerOptions, MemoryStatsStats, StatsOptions}, secret::{ContainerStateStatusEnum, HealthStatusEnum}};
use futures_util::StreamExt;
use lazy_static::lazy_static;
use packet::{daemon_server::event::DSEventPacket, events::{EventData, ServerStatusEvent, ServerStatusType, Stats}};
use tokio::{select, sync::Mutex};
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

use crate::{docker, encryption, SENDER};

lazy_static! {
    static ref CANCELLATION_TOKEN: Arc<Mutex<Option<CancellationToken>>> = Arc::new(Mutex::new(None));
}

pub async fn get_cancellation_token() -> Result<CancellationToken, String> {
    let mut guard = CANCELLATION_TOKEN.lock().await;

    if guard.is_none() {
        guard.replace(super::get_cancellation_token().ok_or("no parent cancellation token provided")?.child_token());
    }

    Ok(guard.as_ref().expect("should NOT be None after Option::replace() call").clone())
}

pub async fn stop_services() -> Result<(), String> {
    get_cancellation_token().await?.cancel();

    let token = CANCELLATION_TOKEN.lock().await.take();
    drop(token);

    Ok(())
}

async fn send_stat(id: u32, stat: bollard::container::Stats) -> Result<(), String> {
    if stat.precpu_stats.system_cpu_usage.is_none() {
        debug!("Skipping sending stats for server {}: precpu_stats.system_cpu_usage is not populated yet (should only take a cycle)", id);
        return Ok(());
    }

    let server = docker::get()?.inspect_container(&format!("ae_sv_{}", id), Some(InspectContainerOptions {
        size: true,
    })).await.map_err(|e| format!("could not inspect container: {}", e))?;

    const GB: f64 = 1_073_741_824.0;

    let status = match server.state.as_ref().ok_or("no state")?.status.ok_or("no status")? {
        ContainerStateStatusEnum::PAUSED => ServerStatusType::Starting,
        ContainerStateStatusEnum::RESTARTING => ServerStatusType::Restarting,
        ContainerStateStatusEnum::REMOVING => ServerStatusType::Stopping,
        ContainerStateStatusEnum::CREATED | ContainerStateStatusEnum::RUNNING => match server.state.as_ref().ok_or("no state")?.health.as_ref().ok_or("no health")?.status.ok_or("no health.status")? {
            HealthStatusEnum::NONE => ServerStatusType::Healthy,
            HealthStatusEnum::EMPTY => ServerStatusType::Healthy,
            HealthStatusEnum::HEALTHY => ServerStatusType::Healthy,
            HealthStatusEnum::STARTING => ServerStatusType::Starting,
            HealthStatusEnum::UNHEALTHY => ServerStatusType::Unhealthy,
        },
        ContainerStateStatusEnum::EXITED | ContainerStateStatusEnum::DEAD | ContainerStateStatusEnum::EMPTY => ServerStatusType::Stopped,
    };

    let server_status = ServerStatusEvent {
        server: id,
        cpu: match status {
            ServerStatusType::Healthy | ServerStatusType::Starting | ServerStatusType::Stopping => Some(Stats {
                used: (stat.cpu_stats.cpu_usage.total_usage as f64 - stat.precpu_stats.cpu_usage.total_usage as f64) / (stat.cpu_stats.system_cpu_usage.ok_or("no cpu_stats.system_cpu_usage")? as f64 - stat.precpu_stats.system_cpu_usage.ok_or("no precpu_stats.system_cpu_usage")? as f64) * (stat.cpu_stats.online_cpus.ok_or("no cpu_stats.online_cpus")? * 100) as f64,
                total: (stat.cpu_stats.online_cpus.ok_or("no cpu_stats.online_cpus")? * 100) as f64,
            }),
            _ => None,
        },
        memory: match status {
            ServerStatusType::Healthy | ServerStatusType::Starting | ServerStatusType::Stopping => Some(Stats {
                used: (stat.memory_stats.usage.ok_or("no memory_stats.usage")? - match stat.memory_stats.stats.ok_or("no memory_stats.stats")? {
                    MemoryStatsStats::V1(v1) => v1.cache,
                    MemoryStatsStats::V2(v2) => v2.file,
                }) as f64 / GB,
                total: stat.memory_stats.limit.ok_or("no memory_stats.limit")? as f64 / GB,
            }),
            _ => None,
        },
        storage: Some(Stats {
            used: server.size_root_fs.ok_or("no size_root_fs")? as f64 / GB,
            total: 100.0, // TODO: make max storage configurable
        }),
        status,
    };

    if SENDER.lock().await.is_some() {
        let packet = DSEventPacket {
            data: EventData::ServerStatus(server_status),
        };

        let packet = match packet.to_packet() {
            Ok(packet) => packet,
            Err(e) => {
                return Err(format!("Error creating packet: {}", e));
            }
        };

        let packet = match encryption::encrypt_packet(packet) {
            Ok(packet) => packet,
            Err(e) => {
                return Err(format!("Error encrypting packet: {}", e));
            }
        };

        if let Some(tx) = SENDER.lock().await.as_ref() {
            match tx.unbounded_send(Message::Text(packet)) {
                Ok(_) => (),
                Err(e) => {
                    return Err(format!("Could not send packet: {}", e));
                }
            }
        }
    }

    Ok(())
}

async fn run(token: CancellationToken, id: u32) -> Result<(), String> {
    let mut stream = docker::get()?.stats(&format!("ae_sv_{}", id), Some(StatsOptions {
        stream: true,
        one_shot: false,
    }));

    while let Some(stat) = stream.next().await {
        if token.is_cancelled() {
            break;
        }

        match stat {
            Ok(stat) => {
                send_stat(id, stat).await?;
            },
            Err(e) => return Err(format!("could not get stat: {}", e))
        }
    }

    Ok(())
}

pub async fn start(id: u32) -> Result<(), String> {
    let token = get_cancellation_token().await?;

    loop {
        select! {
            _ = token.cancelled() => {
                break;
            }
            res = run(token.clone(), id) => {
                match res {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Error in server status: {}", e);
                        continue;
                    }
                }
            }
        }
    }

    debug!("Exiting server status service for server {}", id);

    Ok(())
}
