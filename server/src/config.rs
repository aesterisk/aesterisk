#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub server: Server,
    #[serde(default)]
    pub sockets: Sockets,
    #[serde(default)]
    pub logging: Logging,
}

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

fn load(file: &str) -> Option<Config> {
    let contents = std::fs::read_to_string(file).ok()?;
    toml::from_str(&contents).ok()
}

pub fn load_or_create(file: &str) -> Config {
    let config = load(file).unwrap_or_default();
    save(&config, file);
    config
}
