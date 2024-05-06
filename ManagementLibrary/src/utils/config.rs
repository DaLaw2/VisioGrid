use std::fs;
use tokio::sync::RwLock;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use crate::utils::logging::*;

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
    pub internal_timestamp: u64, //milliseconds
    pub agent_listen_port: u16, //port
    pub http_server_bind_port: u16, //port
    pub dedicated_port_range: [u16; 2], //port range
    pub refresh_interval: u64, //seconds
    pub polling_interval: u64, //milliseconds
    pub bind_retry_duration: u64, //seconds
    pub agent_idle_duration: u64, //seconds
    pub control_channel_timeout: u64, //seconds
    pub data_channel_timeout: u64, //seconds
    pub file_transfer_timeout: u64, //seconds
    pub font_path: String, //path
    pub font_size: f32, //points
    pub border_width: u32, //pixels
    pub border_color: [u8; 3], //RGB
    pub text_color: [u8; 3], //RGB
}

impl Config {
    pub fn new() -> Self {
        //Seriously, the program must be terminated.
        match fs::read_to_string("./management.toml") {
            Ok(toml_string) => {
                match toml::from_str::<ConfigTable>(&toml_string) {
                    Ok(config_table) => {
                        let config = config_table.config;
                        if !Self::validate(&config) {
                            logging_console!(emergency_entry!("Config", "Invalid configuration file"));
                            panic!("Invalid configuration file");
                        }
                        config
                    },
                    Err(err) => {
                        logging_console!(emergency_entry!("Config", "Unable to parse configuration file", format!("Err: {err}")));
                        panic!("Unable to parse configuration file");
                    },
                }
            },
            Err(err) => {
                logging_console!(emergency_entry!("Config", "Configuration file not found", format!("Err: {err}")));
                panic!("Configuration file not found");
            },
        }
    }

    pub async fn now() -> Config {
        CONFIG.read().await.clone()
    }

    pub async fn update(config: Config) {
        *CONFIG.write().await = config
    }

    pub fn validate(config: &Config) -> bool {
        Config::validate_mini_second(config.internal_timestamp)
            && Config::validate_port_range(config.dedicated_port_range)
            && Config::validate_second(config.refresh_interval)
            && Config::validate_mini_second(config.polling_interval)
            && Config::validate_second(config.bind_retry_duration)
            && Config::validate_second(config.agent_idle_duration)
            && Config::validate_second(config.control_channel_timeout)
            && Config::validate_second(config.data_channel_timeout)
            && Config::validate_second(config.file_transfer_timeout)
            && Config::validate_font_size(config.font_size)
            && Config::validate_border_width(config.border_width)
    }

    fn validate_mini_second(second: u64) -> bool {
        second <= 60000
    }

    fn validate_second(second: u64) -> bool {
        second <= 3600
    }

    fn validate_port_range(port: [u16; 2]) -> bool {
        let (start, end) = match (port.get(0), port.get(1)) {
            (Some(start), Some(end)) => (*start, *end),
            _ => (0_u16, 0_u16),
        };
        end > start
    }

    fn validate_border_width(width: u32) -> bool {
        width > 0_u32
    }

    fn validate_font_size(size: f32) -> bool {
        size > 0_f32
    }
}
