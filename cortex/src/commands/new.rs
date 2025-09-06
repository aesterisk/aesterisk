use std::{fs, path::PathBuf};

use crate::config;

pub fn run(name: String, is_api: bool) {
    if name.trim().is_empty() {
        eprintln!("Error: Specification name cannot be empty.");
        return;
    }

    let spec_paths = match config::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load Cortex configuration: {:#?}", e);
            return;
        }
    }
    .paths
    .specs;

    if is_api {
        new_api_spec(name, spec_paths.api);
    } else {
        new_service_spec(name, spec_paths.services);
    }
}

const DEFAULT_SERVICE_TYPES_SPEC: &str = include_str!("../defaults/service/types.toml");
const DEFAULT_SERVICE_RPC_SPEC: &str = include_str!("../defaults/service/rpc.toml");
const DEFAULT_SERVICE_EVENTS_SPEC: &str = include_str!("../defaults/service/events.toml");
const DEFAULT_SERVICE_DATABASE_SPEC: &str = include_str!("../defaults/service/database.toml");

fn new_service_spec(name: String, mut path: PathBuf) {
    path.push(name);

    match fs::create_dir_all(&path) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Failed to create service specification directory: {:#?}", e);
            return;
        }
    }

    match fs::write(path.join("types.toml"), DEFAULT_SERVICE_TYPES_SPEC) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Failed to write service types specification: {:#?}", e);
            return;
        }
    }

    match fs::write(path.join("rpc.toml"), DEFAULT_SERVICE_RPC_SPEC) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Failed to write service RPC specification: {:#?}", e);
            return;
        }
    }

    match fs::write(path.join("events.toml"), DEFAULT_SERVICE_EVENTS_SPEC) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Failed to write service events specification: {:#?}", e);
            return;
        }
    }

    match fs::write(path.join("database.toml"), DEFAULT_SERVICE_DATABASE_SPEC) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Failed to write service database specification: {:#?}", e);
            return;
        }
    }

    println!("Service specification created at: {}", path.display());
}

fn new_api_spec(name: String, path: PathBuf) {}
