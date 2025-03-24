use std::{collections::HashMap, fs::create_dir_all};:
use bollard::{container::{Config, CreateContainerOptions, ListContainersOptions, NetworkingConfig, RemoveContainerOptions, RestartContainerOptions, StartContainerOptions, StopContainerOptions}, image::CreateImageOptions, secret::{ContainerSummary, EndpointIpamConfig, EndpointSettings, HealthConfig, HostConfig, Mount, MountBindOptions, MountTypeEnum, PortBinding, RestartPolicy, RestartPolicyNameEnum}};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use futures_util::StreamExt;
use regex::Regex;
use tracing::debug;

use crate::docker::network;

#[repr(u8)]
pub enum PortProtocol {
    Tcp = 0,
    Udp = 1,
}

impl PortProtocol {
    pub fn name(&self) -> &'static str {
        match self {
            PortProtocol::Tcp => "tcp",
            PortProtocol::Udp => "udp",
        }
    }
}

pub struct AServer {
    pub id: i32,
    pub name: String,
    pub tag: ATag,
    pub envs: Vec<AEnv>,
    pub networks: Vec<AServerNetwork>,
    pub ports: Vec<APort>,
}

pub struct ATag {
	pub id: i32,
	pub name: String,
	pub image: String,
	pub docker_tag: String,
    pub healthcheck: AHealthcheck,
    pub mounts: Vec<AMount>,
    pub env_defs: Vec<AEnvDef>,
}

pub struct AEnv {
    pub id: i32,
    pub key: String,
    pub value: String,
    pub secret: bool,
}

pub struct AServerNetwork {
    pub network_id: i32,
    pub network_subnet: u8,
    pub server_ip: u8,
}

pub struct APort {
    pub port_id: i32,
    pub port_port: u16,
    pub port_protocol: PortProtocol,
    pub port_mapped: u16,
}

pub struct AMount {
    pub id: i32,
    pub container_path: String,
    pub host_path: String,
}

pub struct AHealthcheck {
    pub test: Vec<String>,
    pub interval: u64,
    pub timeout: u64,
    pub retries: u64,
}

pub struct AEnvDef {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub key: String,
    pub secret: bool,
    pub required: bool,
    pub type_: AEnvDefType,
    pub default: Option<String>,
    pub regex: Option<String>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub trim: bool,
}

#[repr(u8)]
pub enum AEnvDefType {
    Boolean = 0,
    Number = 1,
    String = 2,
}

impl From<u8> for AEnvDefType {
    fn from(value: u8) -> Self {
        match value {
            0 => AEnvDefType::Boolean,
            1 => AEnvDefType::Number,
            2 => AEnvDefType::String,
            _ => panic!("Invalid value for AEnvDefType"),
        }
    }
}

impl From<AEnvDefType> for u8 {
    fn from(value: AEnvDefType) -> Self {
        value as u8
    }
}

pub async fn create_server(server: AServer) -> Result<String, String> {
    let envs = server.envs.into_iter().map(|e| (e.key.clone(), e)).collect::<HashMap<_, _>>();

    for env_def in server.tag.env_defs.into_iter() {
        let exists = envs.contains_key(&env_def.key) && !envs.get(&env_def.key).ok_or("env should exist")?.value.is_empty();

        if env_def.required && !exists {
            return Err(format!("Missing required env: {}", env_def.key));
        }

        if exists {
            let env = envs.get(&env_def.key).ok_or("env should exist")?;

            match env_def.type_ {
                AEnvDefType::Boolean => {
                    if env.value != "1" && env.value != "0" {
                        return Err(format!("Invalid value for {}: '{}' is not a boolean value", env_def.key, env.value));
                    }
                },
                AEnvDefType::Number => {
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
                AEnvDefType::String => {
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

        let server_data = format!("./data/{}/", server.id);
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
        networking_config: if let Some(id) = nicc {
            Some(NetworkingConfig {
                endpoints_config: HashMap::from([
                    (id, EndpointSettings::default())
                ])
            })
        } else {
            Some(NetworkingConfig {
                endpoints_config: server.networks.into_iter().map(|network| (format!("ae_nw_{}", network.network_id), EndpointSettings {
                    ipam_config: Some(EndpointIpamConfig {
                        ipv4_address: Some(format!("10.133.{}.{}", network.network_subnet, network.server_ip)),
                        ..Default::default()
                    }),
                    ..Default::default()
                })).collect::<HashMap<_, _>>(),
            })
        },
        host_config: Some(HostConfig {
            network_mode: Some("none".to_string()),
            restart_policy: Some(RestartPolicy {
                name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
                ..Default::default()
            }),
            port_bindings: Some(server.ports.into_iter().map(|port| (format!("{}/{}", port.port_port, port.port_protocol.name()), Some(vec![PortBinding {
                host_ip: Some("".to_string()),
                host_port: Some(format!("{}", port.port_mapped)),
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
    let container = get_server(id).await?.ok_or("Server does not exist")?;
    Ok(super::get()?.restart_container(container.id.as_ref().ok_or("Container should have an ID")?, None::<RestartContainerOptions>).await.is_ok())
}

pub async fn is_running(id: u32) -> Result<bool, String> {
    let container = get_server(id).await?.ok_or("Server does not exist")?;
    Ok(container.state.ok_or("Container should have a state")? == "running")
}
