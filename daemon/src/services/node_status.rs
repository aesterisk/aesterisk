use std::{collections::HashSet, time::Duration};

use packet::{daemon_server::event::DSEventPacket, events::{EventData, EventType, NodeStats, NodeStatusEvent}};
use sysinfo::{CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use tokio::select;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use crate::{encryption, LISTENS, SENDER};

/// Runs the node status service, sending status information to the clients
pub async fn run(token: CancellationToken) -> Result<(), String> {
    select! {
        _ = token.cancelled() => {
            warn!("Stopping node status service");
            Ok(())
        },
        res = send_loop() => {
            res
        }
    }
}

async fn send_loop() -> Result<(), String> {
    // TODO: make this configurable
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut system = System::new();
    let mut disks = Disks::new();

    const GB: f64 = 1_073_741_824.0;

    loop {
        interval.tick().await;

        if !LISTENS.read().await.contains(&EventType::NodeStatus) {
            continue;
        }

        if SENDER.lock().await.is_some() {
            system.refresh_specifics(RefreshKind::nothing().with_memory(MemoryRefreshKind::nothing().with_ram()).with_cpu(CpuRefreshKind::nothing().with_cpu_usage()));
            disks.refresh_specifics(true, DiskRefreshKind::nothing().with_storage());

            let mut counted = HashSet::new();

            let (used, total) = disks.iter()
                .filter(|disk| counted.insert(disk.name().to_string_lossy()))
                .filter(|disk| !disk.is_removable())
                .map(|disk| (disk.available_space(), disk.total_space()))
                .map(|(available, total)| (total - available, total))
                .fold((0, 0), |(used, total), (used2, total2)| (used + used2, total + total2));

            let packet = DSEventPacket {
                data: EventData::NodeStatus(NodeStatusEvent {
                    online: true,
                    stats: Some(NodeStats {
                        used_memory: system.used_memory() as f64 / GB,
                        total_memory: system.total_memory() as f64 / GB,
                        cpu: system.global_cpu_usage() as f64,
                        used_storage: used as f64 / GB,
                        total_storage: total as f64 / GB,
                    }),
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
    }
}
