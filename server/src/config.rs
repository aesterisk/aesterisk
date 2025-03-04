use lazy_static::lazy_static;

lazy_static! {
    pub static ref CONFIG: Config = load_or_create("config.toml");
}

/// The `Config` struct represents the configuration of the server.
#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct Config {
    /// The server configuration.
    #[serde(default)]
    pub server: Server,
    /// The socket configuration.
    #[serde(default)]
    pub sockets: Sockets,
    /// The logging configuration.
    #[serde(default)]
    pub logging: Logging,
}

/// The `Server` struct represents the server configuration.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Server {
    /// The URL of the web (frontend) server.
    pub web_url: String,
    /// The path to the server private key.
    pub private_key: String,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            web_url: "http://127.0.0.1:3000".to_string(),
            private_key: "private.pem".to_string(),
        }
    }
}

/// The `Sockets` struct represents the socket configuration.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Sockets {
    /// The address to bind the web server.
    pub web: String,
    /// The address to bind the daemon server.
    pub daemon: String,
}

impl Default for Sockets {
    fn default() -> Self {
        Self {
            web: "127.0.0.1:31306".to_string(),
            daemon: "127.0.0.1:31304".to_string(),
        }
    }
}

/// The `Logging` struct represents the logging configuration.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Logging {
    /// The folder to store log files in.
    pub folder: String,
}

impl Default for Logging {
    fn default() -> Self {
        Self {
            folder: "./logs".to_string()
        }
    }
}

fn save(config: &Config, file: &str) {
    std::fs::write(file, toml::to_string_pretty(&config).expect("failed to serialize default config")).expect("could not write config file");
}

fn load(file: &str) -> Option<Config> {
    let contents = std::fs::read_to_string(file).ok()?;
    toml::from_str(&contents).ok()
}

/// Load the configuration from the given file, or create the file with the default configuration if
/// it does not exist.
pub fn load_or_create(file: &str) -> Config {
    let config = load(file).unwrap_or_default();
    save(&config, file);
    config
}
