use std::{collections::HashMap, fs::create_dir_all};

use bollard::{container::{Config, CreateContainerOptions, ListContainersOptions, NetworkingConfig, RemoveContainerOptions, RestartContainerOptions, StartContainerOptions, StopContainerOptions}, image::CreateImageOptions, secret::{ContainerSummary, EndpointIpamConfig, EndpointSettings, HealthConfig, HostConfig, Mount, MountBindOptions, MountTypeEnum, PortBinding, RestartPolicy, RestartPolicyNameEnum}};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use futures_util::StreamExt;
use tracing::debug;

use crate::docker::network;

pub struct AEnv {
    pub env_id: i32,
    pub env_key: String,
    pub env_value: String,
    pub env_secret: bool,
}

pub struct AServerNetwork {
    pub network_id: i32,
    pub network_local_ip: i16,
    pub server_local_ip: i16,
}

pub struct APort {
    pub port_id: i32,
    pub port_port: i32,
    pub port_protocol: PortProtocol,
    pub port_mapped: i32,
}

#[repr(i16)]
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
    pub server_id: i32,
    pub server_name: String,
    pub server_tag: i32,
}

pub struct AMount {
    pub mount_id: i32,
    pub mount_container_path: String,
    pub mount_host_path: String,
}

pub struct ATag {
	pub tag_id: i32,
	pub tag_name: String,
	pub tag_image: String,
	pub tag_docker_tags: String,
	pub tag_healthcheck_test: Vec<String>,
	pub tag_healthcheck_interval: i32,
	pub tag_healthcheck_timeout: i32,
	pub tag_healthcheck_retries: i32,
}

pub async fn create_server(server: &AServer, tag: ATag, envs: Vec<AEnv>, networks: Vec<AServerNetwork>, ports: Vec<APort>, mounts: Vec<AMount>) -> Result<String, String> {
    match super::get()?.create_image(Some(CreateImageOptions {
        from_image: tag.tag_image.clone(),
        tag: tag.tag_docker_tags.clone(),
        ..Default::default()
    }), None, None).collect::<Vec<_>>().await.into_iter().reduce(|a, b| a.and(b)) {
        None => (),
        Some(res) => {
            res.map_err(|e| format!("Could not create Docker image: {}", e))?;
        }
    }

    let create_container_options = CreateContainerOptions {
        name: format!("ae_sv_{}", server.server_id),
        ..Default::default()
    };

    let nicc = if networks.is_empty() {
        debug!("Obtaining or creating NICC network");
        Some(network::get_nicc().await?)
    } else {
        None
    };

    let mounts = if !mounts.is_empty() {
        debug!("Validating mounts...");

        let server_data = format!("./data/{}/", server.server_id);
        let data_path = Utf8Path::new(&server_data);

        let _ = create_dir_all(data_path);
        debug!("Data directory created: '{}'", data_path);
        
        let mounts = mounts.into_iter().filter_map(|mount| {
            debug!("Validating mount host path: '{}'...", mount.mount_host_path);
            let unsafe_path = Utf8Path::new(&mount.mount_host_path);
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
                    target: Some(mount.mount_container_path),
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

    debug!("Creating container...");

    let container_config = Config {
        hostname: Some(format!("ae_sv_{}", server.server_id)),
        tty: Some(true),
        env: Some(envs.into_iter().map(|env| format!("{}={}", env.env_key, env.env_value)).collect()),
        image: Some(format!("{}:{}", tag.tag_image, tag.tag_docker_tags)),
        labels: Some(HashMap::from([
            ("io.aesterisk.version".to_string(), "0".to_string()),
            ("io.aesterisk.server.id".to_string(), format!("{}", server.server_id)),
        ])),
        healthcheck: Some(HealthConfig {
            test: Some(tag.tag_healthcheck_test),
            timeout: Some(tag.tag_healthcheck_timeout as i64 * 1_000_000),
            interval: Some(tag.tag_healthcheck_interval as i64 * 1_000_000),
            retries: Some(tag.tag_healthcheck_retries as i64),
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
                endpoints_config: networks.into_iter().map(|network| (format!("ae_nw_{}", network.network_id), EndpointSettings {
                    ipam_config: Some(EndpointIpamConfig {
                        ipv4_address: Some(format!("10.133.{}.{}", network.network_local_ip, network.server_local_ip)),
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
            port_bindings: Some(ports.into_iter().map(|port| (format!("{}/{}", port.port_port, port.port_protocol.name()), Some(vec![PortBinding {
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

pub async fn get_server(id: i32) -> Result<Option<ContainerSummary>, String> {
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

pub async fn server_exists(id: i32) -> Result<bool, String> {
    Ok(get_server(id).await?.is_some())
}

pub async fn stop_server(id: i32) -> Result<bool, String> {
    let container = get_server(id).await?.ok_or("Server does not exist")?;
    Ok(super::get()?.stop_container(container.id.as_ref().ok_or("Container should have an ID")?, None::<StopContainerOptions>).await.is_ok()
        && super::get()?.remove_container(container.id.as_ref().ok_or("Container should have an ID")?, None::<RemoveContainerOptions>).await.is_ok())
}

pub async fn restart_server(id: i32) -> Result<bool, String> {
    let container = get_server(id).await?.ok_or("Server does not exist")?;
    Ok(super::get()?.restart_container(container.id.as_ref().ok_or("Container should have an ID")?, None::<RestartContainerOptions>).await.is_ok())
}

pub async fn is_running(id: i32) -> Result<bool, String> {
    let container = get_server(id).await?.ok_or("Server does not exist")?;
    Ok(container.state.ok_or("Container should have a state")? == "running")
}
