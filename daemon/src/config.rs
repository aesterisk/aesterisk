#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub daemon: Daemon,
    #[serde(default)]
    pub logging: Logging,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Daemon {
    pub id: String,
    pub public_key: String,
    pub private_key: String,
}

impl Default for Daemon {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            public_key: "public.pem".to_string(),
            private_key: "private.pem".to_string(),
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
            folder: "./logs".to_string(),
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
