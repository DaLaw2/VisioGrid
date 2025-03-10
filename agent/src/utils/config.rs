use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::ToSocketAddrs;
use tokio::sync::RwLock;

lazy_static! {
    static ref CONFIG: RwLock<Config> = RwLock::new(Config::new());
}

#[derive(Debug, Deserialize)]
struct ConfigTable {
    #[serde(rename = "Config")]
    config: Config,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub internal_timestamp: u64,
    pub management_address: String,
    pub management_port: u16,
    pub refresh_interval: u64,
    pub polling_interval: u64,
    pub control_channel_timeout: u64,
    pub data_channel_timeout: u64,
    pub file_transfer_timeout: u64,
}

impl Config {
    pub fn new() -> Self {
        //Seriously, the program must be terminated.
        let toml_string = fs::read_to_string("./agent.toml").expect("No configuration found.");
        let config_table: ConfigTable = toml::from_str(&toml_string).expect("Unable parse configuration.");
        let config = config_table.config;
        if !Self::validate(&config) {
            panic!("Invalid configuration.");
        }
        config
    }

    pub async fn now() -> Self {
        CONFIG.read().await.clone()
    }

    pub async fn update(config: Config) {
        *CONFIG.write().await = config
    }

    pub fn validate(config: &Config) -> bool {
        Config::validate_mini_second(config.internal_timestamp)
            && Config::validate_full_address(&config.management_address, config.management_port)
            && Config::validate_second(config.control_channel_timeout)
            && Config::validate_mini_second(config.refresh_interval)
            && Config::validate_second(config.polling_interval)
            && Config::validate_second(config.data_channel_timeout)
            && Config::validate_second(config.file_transfer_timeout)
    }

    fn validate_mini_second(second: u64) -> bool {
        second <= 60000
    }

    fn validate_second(second: u64) -> bool {
        second <= 86400
    }

    fn validate_full_address(address: &str, port: u16) -> bool {
        format!("{}:{}", address, port).to_socket_addrs().is_ok()
    }
}
