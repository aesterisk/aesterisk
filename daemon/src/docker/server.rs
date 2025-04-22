use std::{collections::HashMap, fs::create_dir_all};
use bollard::{container::{Config, CreateContainerOptions, ListContainersOptions, NetworkingConfig, RemoveContainerOptions, RestartContainerOptions, StartContainerOptions, StopContainerOptions}, image::CreateImageOptions, secret::{ContainerSummary, EndpointIpamConfig, EndpointSettings, HealthConfig, HostConfig, MountBindOptions, MountTypeEnum, PortBinding, RestartPolicy, RestartPolicyNameEnum}};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use futures_util::StreamExt;
use packet::server_daemon::sync::{Env, EnvDef, EnvType, Mount, Server, ServerNetwork};
use regex::Regex;
use tracing::debug;

use crate::{config, docker::{self, network}};

fn validate_env_defs(envs: &HashMap<String, Env>, env_defs: Vec<EnvDef>) -> Result<(), String> {
    for env_def in env_defs.into_iter() {
        let exists = envs.contains_key(&env_def.key) && !envs.get(&env_def.key).ok_or("env should exist")?.value.is_empty();

        if !exists {
            return if env_def.required {
                Err(format!("Missing required env: {}", env_def.key))
            } else {
                Ok(())
            }
        }

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
                        // TODO: use `let_chains` when Rust 1.87.0 (most likely) is released
                        // (as of now, the `let_chains` feature was literally merged 4 hours ago...
                        //  what's the odds of that??)
                        if let Some(min) = env_def.min {
                            if num < min {
                                return Err(format!("Invalid value for {}: '{}' is below the minimum value", env_def.key, env.value));
                            }
                        }

                        if let Some(max) = env_def.max {
                            if num > max {
                                return Err(format!("Invalid value for {}: '{}' is above the maximum value", env_def.key, env.value));
                            }
                        }
                    },
                    Err(_) => {
                        return Err(format!("Invalid value for {}: '{}' is not a number", env_def.key, env.value));
                    }
                };
            },
            EnvType::String => {
                let value = if env_def.trim {
                    env.value.trim()
                } else {
                    &env.value
                };

                if let Some(regex) = env_def.regex.as_ref() {
                    let re = Regex::new(regex).map_err(|e| format!("invalid regex: {}", e))?;
                    if !re.is_match(value) {
                        return Err(format!("Invalid value for {}: '{}' does not match regex", env_def.key, env.value));
                    }
                }

                let len = value.len();

                if let Some(min) = env_def.min {
                    if len < min as usize {
                        return Err(format!("Invalid value for {}: '{}' is below the minimum length", env_def.key, env.value));
                    }
                }

                if let Some(max) = env_def.max {
                    if len > max as usize {
                        return Err(format!("Invalid value for {}: '{}' is above the maximum length", env_def.key, env.value));
                    }
                }
            }
        };
    }

    Ok(())
}

fn validate_mounts(server_id: u32, mounts: Vec<Mount>) -> Result<Option<Vec<bollard::models::Mount>>, String> {
    if !mounts.is_empty() {
        debug!("Validating mounts...");

        let server_data = format!("{}/{}/", config::get()?.daemon.data_folder, server_id);
        let data_path = Utf8Path::new(&server_data);

        create_dir_all(data_path).map_err(|e| format!("Could not create data directory: {}", e))?;
        debug!("Data directory created: '{}'", data_path);

        let mounts = mounts.into_iter().filter_map(|mount| {
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
                Some(bollard::models::Mount {
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

        Ok(Some(mounts))
    } else {
        Ok(None)
    }
}

async fn pull_image(image: &str, tag: &str) -> Result<(), String> {
    match super::get()?.create_image(Some(CreateImageOptions {
        from_image: image,
        tag,
        ..Default::default()
    }), None, None).collect::<Vec<_>>().await.into_iter().reduce(|a, b| a.and(b)) {
        None => (),
        Some(res) => {
            res.map_err(|e| format!("Could not create Docker image: {}", e))?;
        }
    }

    Ok(())
}

async fn get_endpoint_config(networks: Vec<ServerNetwork>) -> Result<HashMap<String, EndpointSettings>, String> {
    let nicc = if networks.is_empty() {
        debug!("Obtaining or creating NICC network");
        Some(network::get_nicc().await?)
    } else {
        None
    };

    if let Some(id) = nicc {
        Ok(HashMap::from([
            (id, EndpointSettings::default())
        ]))
    } else {
        let subnets = docker::network::get_networks().await?.into_iter().map(|nw| (nw.id, nw.subnet)).collect::<HashMap<_, _>>();

        let networks = networks.into_iter().map(|nw| Ok((format!("ae_nw_{}", nw.network), EndpointSettings {
            ipam_config: Some(EndpointIpamConfig {
                ipv4_address: Some(format!("10.133.{}.{}", subnets.get(&nw.network).ok_or("network not found")?, nw.ip)),
                ..Default::default()
            }),
            ..Default::default()
        }))).collect::<Result<Vec<_>, String>>()?;

        Ok(networks.into_iter().collect::<HashMap<_, _>>())
    }
}

pub async fn create_server(server: Server) -> Result<String, String> {
    let envs = server.envs.into_iter().map(|e| (e.key.clone(), e)).collect::<HashMap<_, _>>();

    validate_env_defs(&envs, server.tag.env_defs).map_err(|e| format!("Failed to validate env defs: {}", e))?;

    let create_container_options = CreateContainerOptions {
        name: format!("ae_sv_{}", server.id),
        ..Default::default()
    };

    let mounts = validate_mounts(server.id, server.tag.mounts).map_err(|e| format!("Failed to validate mounts: {}", e))?;

    pull_image(&server.tag.image, &server.tag.docker_tag).await.map_err(|e| format!("Failed to pull image: {}", e))?;

    debug!("Creating container...");

    let endpoints_config = get_endpoint_config(server.networks).await.map_err(|e| format!("Failed to get endpoint config: {}", e))?;

    let container_config = Config {
        hostname: Some(format!("ae_sv_{}", server.id)),
        tty: Some(true),
        env: Some(envs.values().map(|env| format!("{}={}", env.key, env.value)).collect()),
        image: Some(format!("{}:{}", server.tag.image, server.tag.docker_tag)),
        labels: Some(HashMap::from([
            ("io.aesterisk.server.version".to_string(), "0".to_string()),
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

    Ok(id)
}

pub async fn get_servers() -> Result<Vec<ContainerSummary>, String> {
    let list_containers_options = ListContainersOptions {
        all: true,
        filters: HashMap::from([
            ("label".to_string(), vec![
                "io.aesterisk.server.version=0".to_string()
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
                "io.aesterisk.server.version=0".to_string()
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
