#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub server: Server,
    pub sockets: Sockets,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Server {
    pub app_url: String,
    pub private_key: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Sockets {
    pub app: String,
    pub daemon: String,
}

fn create_defaults(file: &str) -> Config {
    let config = Config {
        server: Server {
            app_url: "http://localhost:3000".to_string(),
            private_key: "server.pem".to_string(),
        },
        sockets: Sockets {
            app: "127.0.0.1:31306".to_string(),
            daemon: "127.0.0.1:31304".to_string(),
        }
    };

    save(&config, file);

    config
}

fn save(config: &Config, file: &str) {
    std::fs::write(file, toml::to_string_pretty(&config).expect("failed to serialize default config")).expect("could not write config file");
}

fn load(file: &str) -> Option<Config> {
    let contents = std::fs::read_to_string(file).ok()?;
    toml::from_str(&contents).ok()
}

pub fn load_or_create(file: &str) -> Config {
    load(file).unwrap_or_else(|| create_defaults(file))
}
