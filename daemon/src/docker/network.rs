use std::collections::HashMap;

use bollard::{network::{CreateNetworkOptions, ListNetworksOptions}, secret::{Ipam, IpamConfig, Network}};
use tracing::debug;

pub async fn create_network(id: u32, subnet: u8) -> Result<String, String> {
    let ipam_config = IpamConfig {
        subnet: Some(format!("10.133.{}.0/24", subnet)),
        ..Default::default()
    };

    let create_network_options = CreateNetworkOptions {
        name: format!("ae_nw_{}", id),
        check_duplicate: true,
        driver: "bridge".into(),
        ipam: Ipam {
            config: Some(vec![ipam_config]),
            ..Default::default()
        },
        labels: HashMap::from([
            ("io.aesterisk.version".to_string(), "0".to_string()),
            ("io.aesterisk.network.id".to_string(), format!("{}", id)),
        ]),
        ..Default::default()
    };

    Ok(super::get()?.create_network(create_network_options).await.expect("could not create docker network").id)
}

pub async fn get_networks() -> Result<Vec<Network>, String> {
    let list_networks_options = ListNetworksOptions {
        filters: HashMap::from([
            ("label".to_string(), vec![
                "io.aesterisk.version".to_string(),
                "io.aesterisk.nicc=0".to_string(),
            ]),
        ]),
    };

    super::get()?.list_networks(Some(list_networks_options)).await.map_err(|e| format!("Could not get networks from Docker: {}", e))
}

async fn get_network(id: u32) -> Result<Option<Network>, String> {
    let list_networks_options = ListNetworksOptions {
        filters: HashMap::from([
            ("label".to_string(), vec![
                format!("io.aesterisk.network.id={}", id),
                "io.aesterisk.version=0".to_string()
            ]),
        ]),
    };

    Ok(super::get()?.list_networks(Some(list_networks_options)).await.map_err(|e| format!("Could not get networks from Docker: {}", e))?.into_iter().next())
}

pub async fn network_exists(id: u32) -> Result<bool, String> {
    Ok(get_network(id).await?.is_some())
}

pub async fn delete_network(id: u32) -> Result<String, String> {
    let network = get_network(id).await?;

    if network.is_none() {
        return Err("Network does not exist".to_string());
    }

    let network = network.unwrap();
    let id = network.id.ok_or("Found network has no ID")?;

    super::get()?.remove_network(&id).await.map_err(|e| format!("Could not remove Docker network: {}", e))?;

    Ok(id)
}

pub async fn get_nicc() -> Result<String, String> {
    let list_networks_options = ListNetworksOptions {
        filters: HashMap::from([
            ("label".to_string(), vec![
                "io.aesterisk.version=0".to_string(),
                "io.aesterisk.nicc=1".to_string(),
            ]),
        ]),
    };

    match super::get()?.list_networks(Some(list_networks_options)).await.map_err(|e| format!("Could not get networks from Docker: {}", e))?.into_iter().next() {
        Some(nicc) => Ok(nicc.id.ok_or("NICC has no ID")?),
        None => Ok(create_nicc().await?),
    }
}

async fn create_nicc() -> Result<String, String> {
    let create_network_options = CreateNetworkOptions {
        name: "ae_nicc".to_string(),
        check_duplicate: true,
        driver: "bridge".to_string(),
        labels: HashMap::from([
            ("io.aesterisk.version".to_string(), "0".to_string()),
            ("io.aesterisk.nicc".to_string(), "1".to_string()),
        ]),
        options: HashMap::from([
            ("com.docker.network.bridge.enable_icc".to_string(), "false".to_string())
        ]),
        ..Default::default()
    };

    debug!("Creating NICC network...");

    Ok(super::get()?.create_network(create_network_options).await.map_err(|e| format!("Could not create NICC network: {}", e))?.id)
}
