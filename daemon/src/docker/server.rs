use std::{collections::HashMap, fs::create_dir_all};
use bollard::{container::{Config, CreateContainerOptions, InspectContainerOptions, ListContainersOptions, NetworkingConfig, RemoveContainerOptions, RestartContainerOptions, StartContainerOptions, StatsOptions, StopContainerOptions}, image::CreateImageOptions, secret::{ContainerStateStatusEnum, ContainerSummary, EndpointIpamConfig, EndpointSettings, HealthConfig, HealthStatusEnum, HostConfig, Mount, MountBindOptions, MountTypeEnum, PortBinding, RestartPolicy, RestartPolicyNameEnum}};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use futures_util::StreamExt;
use packet::{daemon_server::event::DSEventPacket, events::{EventData, ServerStatus, ServerStatusEvent, ServerStatusType, Stats}, server_daemon::sync::{EnvType, Server}};
use regex::Regex;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error};

use crate::{docker::{self, network}, encryption, SENDER};

pub async fn create_server(server: Server) -> Result<String, String> {
    let envs = server.envs.into_iter().map(|e| (e.key.clone(), e)).collect::<HashMap<_, _>>();

    for env_def in server.tag.env_defs.into_iter() {
        let exists = envs.contains_key(&env_def.key) && !envs.get(&env_def.key).ok_or("env should exist")?.value.is_empty();

        if env_def.required && !exists {
            return Err(format!("Missing required env: {}", env_def.key));
        }

        if exists {
            let env = envs.get(&env_def.key).ok_or("env should exist")?;

            match env_def.env_type {
                EnvType::Boolean => {
                    if env.value != "1" && env.value != "0" {
                        return Err(format!("Invalid value for {}: '{}' is not a boolean value", env_def.key, env.value));
                    }
                },
                EnvType::Number => {
                    let parsed = env.value.parse::<i64>();
                    match parsed {
                        Ok(num) => {
                            if env_def.min.is_some() && num < env_def.min.ok_or("env should have min")? {
                                return Err(format!("Invalid value for {}: '{}' is below the minimum value", env_def.key, env.value));
                            }

                            if env_def.max.is_some() && num > env_def.max.ok_or("env should have max")? {
                                return Err(format!("  Invalid value for {}: '{}' is above the maximum value", env_def.key, env.value));
                            }
                        },
                        Err(_) => {
                            return Err(format!("  Invalid value for {}: '{}' is not a number", env_def.key, env.value));
                        }
                    };
                },
                EnvType::String => {
                    let value = if env_def.trim {
                        env.value.trim()
                    } else {
                        &env.value
                    };

                    if env_def.regex.is_some() {
                        let re = Regex::new(env_def.regex.as_ref().ok_or("env should have regex")?).map_err(|_| "invalid regex")?;
                        if !re.is_match(value) {
                            return Err(format!("  Invalid value for {}: '{}' does not match regex", env_def.key, env.value));
                        }
                    }

                    let len = value.len();

                    if env_def.min.is_some() && len < env_def.min.ok_or("env should have min")? as usize {
                        return Err(format!("  Invalid value for {}: '{}' is below the minimum length", env_def.key, env.value));
                    }

                    if env_def.max.is_some() && len > env_def.max.ok_or("env should have max")? as usize {
                        return Err(format!("  Invalid value for {}: '{}' is above the maximum length", env_def.key, env.value));
                    }
                }
            };
        }
    }

    let create_container_options = CreateContainerOptions {
        name: format!("ae_sv_{}", server.id),
        ..Default::default()
    };

    let nicc = if server.networks.is_empty() {
        debug!("Obtaining or creating NICC network");
        Some(network::get_nicc().await?)
    } else {
        None
    };

    let mounts = if !server.tag.mounts.is_empty() {
        debug!("Validating mounts...");

        let server_data = format!("/tmp/aesterisk/data/{}/", server.id);
        let data_path = Utf8Path::new(&server_data);

        let _ = create_dir_all(data_path);
        debug!("Data directory created: '{}'", data_path);

        let mounts = server.tag.mounts.into_iter().filter_map(|mount| {
            debug!("Validating mount host path: '{}'...", mount.host_path);
            let unsafe_path = Utf8Path::new(&mount.host_path);
            let safe_path = unsafe_path.strip_prefix("/").unwrap_or(unsafe_path);
            let joined_path = data_path.join(safe_path);

            let mut components = vec![];

            for component in joined_path.components() {
                match component {
                    Utf8Component::ParentDir => {
                        if let Some(Utf8Component::Normal(_)) = components.last() {
                            components.pop();
                        } else {
                            components.push(component);
                        }
                    },
                    _ => components.push(component),
                }
            }

            let path = components.iter().collect::<Utf8PathBuf>();

            if path.starts_with(data_path) {
                debug!("Mount validated successfully");
                Some(Mount {
                    target: Some(mount.container_path),
                    source: Some(path.into_string()),
                    typ: Some(MountTypeEnum::BIND),
                    bind_options: Some(MountBindOptions {
                        create_mountpoint: Some(true),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
            } else {
                debug!("Mount is invalid, skipping");
                None
            }
        }).collect::<Vec<_>>();

        debug!("Mounts validated");

        Some(mounts)
    } else {
        None
    };

    match super::get()?.create_image(Some(CreateImageOptions {
        from_image: server.tag.image.clone(),
        tag: server.tag.docker_tag.clone(),
        ..Default::default()
    }), None, None).collect::<Vec<_>>().await.into_iter().reduce(|a, b| a.and(b)) {
        None => (),
        Some(res) => {
            res.map_err(|e| format!("Could not create Docker image: {}", e))?;
        }
    }

    debug!("Creating container...");

    let endpoints_config: Result<HashMap<_, _>, String> = if let Some(id) = nicc {
        Ok(HashMap::from([
            (id, EndpointSettings::default())
        ]))
    } else {
        let subnets = docker::network::get_networks().await?.into_iter().map(|nw| (nw.id, nw.subnet)).collect::<HashMap<_, _>>();

        let networks = server.networks.into_iter().map(|nw| Ok((format!("ae_nw_{}", nw.network), EndpointSettings {
            ipam_config: Some(EndpointIpamConfig {
                ipv4_address: Some(format!("10.133.{}.{}", subnets.get(&nw.network).ok_or("network not found")?, nw.ip)),
                ..Default::default()
            }),
            ..Default::default()
        }))).collect::<Result<Vec<_>, String>>()?;

        Ok(networks.into_iter().collect::<HashMap<_, _>>())
    };

    let endpoints_config = endpoints_config?;

    let container_config = Config {
        hostname: Some(format!("ae_sv_{}", server.id)),
        tty: Some(true),
        env: Some(envs.values().map(|env| format!("{}={}", env.key, env.value)).collect()),
        image: Some(format!("{}:{}", server.tag.image, server.tag.docker_tag)),
        labels: Some(HashMap::from([
            ("io.aesterisk.version".to_string(), "0".to_string()),
            ("io.aesterisk.server.id".to_string(), format!("{}", server.id)),
        ])),
        healthcheck: Some(HealthConfig {
            test: Some(server.tag.healthcheck.test),
            timeout: Some(server.tag.healthcheck.timeout as i64 * 1_000_000),
            interval: Some(server.tag.healthcheck.interval as i64 * 1_000_000),
            retries: Some(server.tag.healthcheck.retries as i64),
            ..Default::default()
        }),
        networking_config: Some(NetworkingConfig {
            endpoints_config,
        }),
        host_config: Some(HostConfig {
            network_mode: Some("none".to_string()),
            restart_policy: Some(RestartPolicy {
                name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
                ..Default::default()
            }),
            port_bindings: Some(server.ports.into_iter().map(|port| (format!("{}/{}", port.port, port.protocol), Some(vec![PortBinding {
                host_ip: Some("".to_string()),
                host_port: Some(format!("{}", port.mapped)),
            }]))).collect::<HashMap<_, _>>()),
            mounts,
            ..Default::default()
        }),
        ..Default::default()
    };

    let id = super::get()?.create_container(Some(create_container_options), container_config).await.map_err(|e| format!("Could not create Docker container: {}", e))?.id;

    debug!("Created container: '{}'", id);

    debug!("Starting container...");

    super::get()?.start_container(&id, None::<StartContainerOptions<String>>).await.map_err(|e| format!("Could not start Docker container: {}", e))?;

    debug!("Started container");

    let id_clone = id.clone();
    let server_id = server.id;
    tokio::spawn(async move {
        let send_stat = async |id: u32, docker_id: &str, stat: bollard::container::Stats| -> Result<(), String> {
            let server = super::get()?.inspect_container(docker_id, Some(InspectContainerOptions {
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
                ContainerStateStatusEnum::EXITED | ContainerStateStatusEnum::DEAD | ContainerStateStatusEnum::EMPTY => ServerStatusType::Offline,
            };

            let server_status = ServerStatus {
                server: id,
                cpu: match status {
                    ServerStatusType::Healthy | ServerStatusType::Starting | ServerStatusType::Stopping => Some(Stats {
                        used: stat.cpu_stats.cpu_usage.total_usage as f64,
                        total: stat.cpu_stats.online_cpus.ok_or("no cpu_stats.online_cpus")? as f64,
                    }),
                    _ => None,
                },
                memory: match status {
                    ServerStatusType::Healthy | ServerStatusType::Starting | ServerStatusType::Stopping => Some(Stats {
                        used: stat.memory_stats.usage.ok_or("no memory_stats.usage")? as f64 / GB,
                        total: stat.memory_stats.limit.ok_or("no memory_stats.usage")? as f64 / GB,
                    }),
                    _ => None,
                },
                storage: Some(Stats {
                    used: server.size_root_fs.ok_or("no size_root_fs")? as f64 / GB,
                    total: 100.0,
                }),
                status,
            };

            if SENDER.lock().await.is_some() {
                let packet = DSEventPacket {
                    data: EventData::ServerStatus(ServerStatusEvent {
                        statuses: vec![server_status],
                    }),
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
        };

        let run = async || -> Result<(), String> {
            let mut stream = super::get()?.stats(&id_clone, Some(StatsOptions {
                stream: true,
                one_shot: false,
            }));

            while let Some(stat) = stream.next().await {
                match stat {
                    Ok(stat) => {
                        send_stat(server_id, &id_clone, stat).await?;
                    },
                    Err(e) => return Err(format!("could not get stat: {}", e))
                }
            }

            Ok(())
        };

        match run().await {
            Ok(()) => (),
            Err(e) => error!("Failed to poll server resource usage: {}", e),
        }
    });

    Ok(id)
}

pub async fn get_servers() -> Result<Vec<ContainerSummary>, String> {
    let list_containers_options = ListContainersOptions {
        all: true,
        filters: HashMap::from([
            ("label".to_string(), vec![
                "io.aesterisk.version=0".to_string()
            ]),
        ]),
        ..Default::default()
    };

    super::get()?.list_containers(Some(list_containers_options)).await.map_err(|e| format!("Could not get containers from Docker: {}", e))
}

pub async fn get_server(id: u32) -> Result<Option<ContainerSummary>, String> {
    let list_containers_options = ListContainersOptions {
        all: true,
        filters: HashMap::from([
            ("label".to_string(), vec![
                format!("io.aesterisk.server.id={}", id),
                "io.aesterisk.version=0".to_string()
            ]),
        ]),
        ..Default::default()
    };

    Ok(super::get()?.list_containers(Some(list_containers_options)).await.map_err(|e| format!("Could not get containers from Docker: {}", e))?.into_iter().next())
}

pub async fn server_exists(id: u32) -> Result<bool, String> {
    Ok(get_server(id).await?.is_some())
}

pub async fn stop_server(id: u32) -> Result<bool, String> {
    let container = get_server(id).await?.ok_or("Server does not exist")?;
    Ok(super::get()?.stop_container(container.id.as_ref().ok_or("Container should have an ID")?, None::<StopContainerOptions>).await.is_ok()
        && super::get()?.remove_container(container.id.as_ref().ok_or("Container should have an ID")?, None::<RemoveContainerOptions>).await.is_ok())
}

pub async fn restart_server(id: u32) -> Result<bool, String> {
    // TODO: change restart_container to stop_container followed by start_container, where
    // start_container (or this function in between) somehow needs to know if there are changes to
    // the server that should be used for the start_container call.

    let container = get_server(id).await?.ok_or("Server does not exist")?;
    Ok(super::get()?.restart_container(container.id.as_ref().ok_or("Container should have an ID")?, None::<RestartContainerOptions>).await.is_ok())
}

pub async fn is_running(id: u32) -> Result<bool, String> {
    let container = get_server(id).await?.ok_or("Server does not exist")?;
    Ok(container.state.ok_or("Container should have a state")? == "running")
}
