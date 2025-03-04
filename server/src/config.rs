use lazy_static::lazy_static;

lazy_static! {
    pub static ref CONFIG: Config = load_or_create("config.toml");
}

/// The `Config` struct represents the configuration of the server.
#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub server: Server,
    #[serde(default)]
    pub sockets: Sockets,
    #[serde(default)]
    pub logging: Logging,
}

/// The `Server` struct represents the server configuration.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Server {
    pub web_url: String,
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
    pub web: String,
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

/// Attempts to load a configuration from a TOML file.
///
/// Reads the file at the provided path and tries to deserialize its contents into a `Config` instance.
/// Returns `None` if the file cannot be read or its contents fail to parse.
///
/// # Arguments
///
/// * `file` - The path to the TOML configuration file.
///
/// # Examples
///
/// ```
/// // Assuming "config.toml" exists and contains valid TOML for a `Config`.
/// if let Some(config) = load("config.toml") {
///     println!("Configuration loaded successfully.");
/// } else {
///     eprintln!("Failed to load configuration.");
/// }
/// ```
fn load(file: &str) -> Option<Config> {
    let contents = std::fs::read_to_string(file).ok()?;
    toml::from_str(&contents).ok()
}

/// Load the configuration from the given file, or create the file with the default configuration if
/// Loads a configuration from the given file or creates a default configuration if loading fails.
/// 
/// This function attempts to load a configuration from the specified file path using the `load` function.
/// If loading is unsuccessful, it falls back to a default configuration. In both cases, the configuration is
/// saved back to the file before being returned.
///
/// # Arguments
///
/// * `file` - A string slice that holds the path to the TOML configuration file.
///
/// # Examples
///
/// ```
/// use your_crate::config::load_or_create;
///
/// let config = load_or_create("config.toml");
/// // If the configuration file is missing or invalid, a default configuration is created and saved.
/// assert!(!config.server.web_url.is_empty());
/// ```pub fn load_or_create(file: &str) -> Config {
    let config = load(file).unwrap_or_default();
    save(&config, file);
    config
}
