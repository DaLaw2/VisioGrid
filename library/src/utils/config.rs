use std::fs;
use tokio::sync::RwLock;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

lazy_static! {
    static ref GLOBAL_CONFIG: RwLock<Config> = RwLock::new(Config::new());
}

#[derive(Debug, Deserialize)]
struct ConfigTable {
    #[serde(rename = "Config")]
    config: Config
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub(crate) http_server_bind_port: usize,
    pub(crate) node_listen_port: usize,
    pub(crate) bind_retry_duration: usize,
    pub(crate) dedicated_port_range: (usize, usize),
}

impl Config {
    fn new() -> Self {
        //Impossible error
        let toml_string = fs::read_to_string("./Config.toml").expect("No configuration found.");
        let config_table: ConfigTable = toml::from_str(&toml_string).expect("Fail parse configuration.");
        config_table.config
    }

    pub async fn instance() -> Config {
        GLOBAL_CONFIG.read().await.clone()
    }

    pub async fn update(config: Config) {
        *GLOBAL_CONFIG.write().await = config
    }
}
