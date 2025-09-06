use std::io;
use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "Cortex.toml";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub paths: Paths,
    pub crates: Option<Crates>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Paths {
    pub api: PathBuf,
    pub services: PathBuf,
    pub clients: PathBuf,
    pub specs: Specifications,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Specifications {
    pub api: PathBuf,
    pub services: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Crates {
    pub shared: Vec<PathBuf>,
}

fn load_from_string(s: &str) -> Result<Config, io::Error> {
    let document = s
        .parse::<toml_edit::DocumentMut>()
        .map_err(io::Error::other)?;
    let config = toml_edit::de::from_document::<Config>(document).map_err(io::Error::other)?;

    Ok(config)
}

pub fn load() -> Result<Config, io::Error> {
    let config_path = PathBuf::from(CONFIG_FILE);
    let exists = config_path.try_exists()?;

    if exists {
        let config_content = fs::read_to_string(config_path)?;
        load_from_string(&config_content)
    } else {
        Err(io::Error::from(io::ErrorKind::NotFound))
    }
}

const DEFAULT_CONFIG: &str = include_str!("./defaults/config.toml");

pub fn save_default() -> Result<(), io::Error> {
    let config_path = PathBuf::from(CONFIG_FILE);
    fs::write(config_path, DEFAULT_CONFIG)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_config() {
        let loaded = load_from_string(DEFAULT_CONFIG);
        assert!(
            loaded.is_ok(),
            "Failed to parse default config: {:?}",
            loaded.err()
        );
    }
}
