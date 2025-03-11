use std::sync::OnceLock;

use tracing::warn;

use crate::Cli;

trait ConfigOverride {
    fn override_with(self, args: &mut Cli) -> Self;
}

/// Configuration file for the daemon
#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct Config {
    /// Daemon configuration
    #[serde(default)]
    pub daemon: Daemon,
    /// Server configuration
    #[serde(default)]
    pub server: Server,
    /// Logging configuration
    #[serde(default)]
    pub logging: Logging,
}

impl ConfigOverride for Config {
    fn override_with(self, args: &mut Cli) -> Self {
        Self {
            daemon: self.daemon.override_with(args),
            server: self.server.override_with(args),
            logging: self.logging.override_with(args),
        }
    }
}

/// Daemon configuration
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Daemon {
    /// Daemon ID
    pub uuid: String,
    /// Path to the daemon's public key
    pub public_key: String,
    /// Path to the daemon's private key
    pub private_key: String,
}

impl Default for Daemon {
    fn default() -> Self {
        Self {
            uuid: "".to_string(),
            public_key: "daemon.pub".to_string(),
            private_key: "daemon.pem".to_string(),
        }
    }
}

impl ConfigOverride for Daemon {
    fn override_with(self, args: &mut Cli) -> Self {
        Self {
            uuid: args.daemon_uuid.take().unwrap_or(self.uuid),
            public_key: args.daemon_public_key.take().unwrap_or(self.public_key),
            private_key: args.daemon_private_key.take().unwrap_or(self.private_key),
        }
    }
}

/// Server configuration
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Server {
    /// Server URL
    pub url: String,
    /// Path to the server's public key
    pub public_key: String,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            url: "wss://daemon.server.aesterisk.io".to_string(),
            public_key: "server.pub".to_string(),
        }
    }
}

impl ConfigOverride for Server {
    fn override_with(self, args: &mut Cli) -> Self {
        Self {
            url: args.server_url.take().unwrap_or(self.url),
            public_key: args.server_public_key.take().unwrap_or(self.public_key),
        }
    }
}

/// Logging configuration
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Logging {
    /// Path to the logs folder
    pub folder: String,
}

impl Default for Logging {
    fn default() -> Self {
        Self {
            folder: "./logs".to_string(),
        }
    }
}

impl ConfigOverride for Logging {
    fn override_with(self, args: &mut Cli) -> Self {
        Self {
            folder: args.logging_folder.take().unwrap_or(self.folder),
        }
    }
}

static CONFIG: OnceLock<Config> = OnceLock::new();

fn save(config: &Config, file: &str) -> Result<(), String> {
    std::fs::write(file, toml::to_string_pretty(&config).map_err(|_| "could not serialize config")?).map_err(|_| "could not write config file")?;
    Ok(())
}

fn load(file: &str) -> Result<Config, String> {
    match std::fs::read_to_string(file) {
        Ok(contents) => Ok(toml::from_str(&contents).map_err(|_| "could not parse config file")?),
        Err(_) => {
            warn!("Could not read config file, generating default configuration");
            Ok(Config::default())
        }
    }
}

fn load_or_create(file: &str) -> Result<Config, String> {
    let config = load(file)?;
    save(&config, file)?;
    Ok(config)
}

pub fn init(default_file: &str, mut override_args: Cli) -> Result<&'static Config, String> {
    if CONFIG.get().is_some() {
        return Err("config already initialized".to_string());
    }

    let config = load_or_create(override_args.config.as_deref().unwrap_or(default_file))?;

    Ok(CONFIG.get_or_init(|| config.override_with(&mut override_args)))
}

pub fn get() -> Result<&'static Config, String> {
    CONFIG.get().ok_or("config not initialized".to_string())
}
