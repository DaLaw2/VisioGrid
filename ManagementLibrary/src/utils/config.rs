use crate::utils::logging::*;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fs;
use tokio::sync::RwLock as AsyncRwLock;
use std::sync::RwLock as SyncRwLock;

lazy_static! {
    static ref ASYNC_CONFIG: AsyncRwLock<Config> = AsyncRwLock::new(Config::new());
    static ref SYNC_CONFIG: SyncRwLock<Config> = SyncRwLock::new(Config::new());
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum SplitMode {
    Frame,
    Time {
        segment_duration_secs: u64, //seconds
    },
}

#[derive(Debug, Deserialize)]
struct ConfigTable {
    #[serde(rename = "Config")]
    config: Config,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub split_mode: SplitMode, //video split mode
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
}

impl Config {
    pub fn new() -> Self {
        let config = match fs::read_to_string("./management.toml") {
            Ok(toml_string) => {
                match toml::from_str::<ConfigTable>(&toml_string) {
                    Ok(config_table) => {
                        let config = config_table.config;
                        if !Self::validate(&config) {
                            logging_console!(emergency_entry!(SystemEntry::InvalidConfig));
                            panic!("{}", SystemEntry::InvalidConfig.to_string());
                        }
                        config
                    },
                    Err(_) => {
                        logging_console!(emergency_entry!(SystemEntry::InvalidConfig));
                        panic!("{}", SystemEntry::InvalidConfig.to_string());
                    },
                }
            },
            Err(_) => {
                logging_console!(emergency_entry!(SystemEntry::ConfigNotFound));
                panic!("{}", SystemEntry::ConfigNotFound.to_string());
            },
        };
        config
    }

    pub async fn now() -> Config {
        ASYNC_CONFIG.read().await.clone()
    }

    pub fn now_blocking() -> Config {
        SYNC_CONFIG.read().unwrap().clone()
    }

    pub async fn update(config: Config) {
        *SYNC_CONFIG.write().unwrap() = config.clone();
        *ASYNC_CONFIG.write().await = config;
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
}
