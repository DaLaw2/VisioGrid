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
    config: Config,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    node_ip: [u8; 4],
    node_listen_port: u16,
}

impl Config {
    pub fn new() -> Self {
        //Seriously, the program must be terminated.
        let toml_string = fs::read_to_string("./client_config.toml").expect("No configuration found.");
        let config_table: ConfigTable = toml::from_str(&toml_string).expect("Unable parse configuration.");
        let config = config_table.config;
        if !Self::validate(&config) {
            panic!("Invalid configuration.");
        }
        config
    }

    pub async fn now() -> Config {
        GLOBAL_CONFIG.read().await.clone()
    }

    pub async fn update(config: Config) {
        *GLOBAL_CONFIG.write().await = config
    }

    pub fn validate(config: &Config) -> bool {
        true
    }

    fn validate_mini_second(second: u64) -> bool {
        second <= 60000
    }

    fn validate_second(second: u64) -> bool {
        second <= 86400
    }
}
