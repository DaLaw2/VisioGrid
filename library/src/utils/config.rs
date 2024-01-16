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
    pub internal_timestamp: u64,
    pub node_listen_port: u16,
    pub http_server_bind_port: u16,
    pub bind_retry_duration: u64,
    pub node_idle_duration: u64,
    pub polling_interval: u64,
    pub control_channel_timeout: u64,
    pub data_channel_timeout: u64,
    pub file_transfer_timeout: u64,
    pub dedicated_port_range: [u16; 2],
    pub font_path: String,
    pub border_width: u32,
    pub font_size: f32,
    pub border_color: [u8; 3],
    pub text_color: [u8; 3],
}

impl Config {
    pub fn new() -> Self {
        //Seriously, the program must be terminated.
        let toml_string = fs::read_to_string("./Config.toml").expect("No configuration found.");
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
            && Config::validate_border_width(config.border_width)
            && Config::validate_font_size(config.font_size)
    }

    fn validate_mini_second(second: u64) -> bool {
        second <= 60000
    }

    fn validate_second(second: u64) -> bool {
        second <= 86400
    }

    fn validate_port(port: u16) -> bool {
        port <= 65535
    }

    fn validate_port_range(port: [u16; 2]) -> bool {
        let (start, end) = match (port.get(0), port.get(1)) {
            (Some(start), Some(end)) => (*start, *end),
            _ => (0_u16, 0_u16),
        };
        Self::validate_port(start) && Self::validate_port(end) && end > start
    }

    fn validate_border_width(width: u32) -> bool {
        width > 0_u32
    }

    fn validate_font_size(size: f32) -> bool {
        size > 0_f32
    }
}
