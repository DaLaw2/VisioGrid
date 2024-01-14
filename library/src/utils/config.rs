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
    pub internal_timestamp: usize,
    pub node_listen_port: usize,
    pub http_server_bind_port: usize,
    pub bind_retry_duration: usize,
    pub node_idle_duration: usize,
    pub polling_interval: usize,
    pub control_channel_timeout: usize,
    pub data_channel_timeout: usize,
    pub file_transfer_timeout: usize,
    pub dedicated_port_range: (usize, usize),
    pub font_path: String,
    pub border_width: usize,
    pub font_size: usize,
    pub border_color: (usize, usize, usize, usize),
    pub text_color: (usize, usize, usize, usize),
}

impl Config {
    pub fn new() -> Self {
        //Seriously, the program must be terminated.
        let toml_string = fs::read_to_string("./Config.toml").expect("No configuration found.");
        let config_table: ConfigTable = toml::from_str(&toml_string).expect("Unable parse configuration.");
        if !Self::validate(&config_table.config) {
            panic!("Invalid configuration.");
        }
        config_table.config
    }

    pub async fn now() -> Config {
        GLOBAL_CONFIG.read().await.clone()
    }

    pub async fn update(config: Config) {
        *GLOBAL_CONFIG.write().await = config
    }

    pub fn validate(config: &Config) -> bool {
        Config::validate_mini_second(config.internal_timestamp)
            && Config::validate_port(config.node_listen_port)
            && Config::validate_port(config.http_server_bind_port)
            && Config::validate_second(config.bind_retry_duration)
            && Config::validate_second(config.node_idle_duration)
            && Config::validate_mini_second(config.polling_interval)
            && Config::validate_second(config.control_channel_timeout)
            && Config::validate_second(config.data_channel_timeout)
            && Config::validate_second(config.file_transfer_timeout)
            && Config::validate_port_range(config.dedicated_port_range)
            && Config::validate_size(config.border_width)
            && Config::validate_size(config.font_size)
            && Config::validate_rgba(config.border_color)
            && Config::validate_rgba(config.text_color)
    }

    fn validate_mini_second(second: usize) -> bool {
        second <= 60000
    }

    fn validate_second(second: usize) -> bool {
        second <= 86400
    }

    fn validate_port(port: usize) -> bool {
        port <= 65535
    }

    fn validate_port_range(port: (usize, usize)) -> bool {
        Self::validate_port(port.0) && Self::validate_port(port.1) && port.1 > port.0
    }

    fn validate_size(size: usize) -> bool {
        size >= 1
    }

    fn validate_rgba(rgba: (usize, usize, usize, usize)) -> bool {
        rgba.0 <= 255 && rgba.1 <= 255 && rgba.2 <= 255 && rgba.3 <= 255
    }
}
