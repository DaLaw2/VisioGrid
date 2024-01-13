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
        config_table.config
    }

    pub async fn now() -> Config {
        GLOBAL_CONFIG.read().await.clone()
    }

    pub async fn update(config: Config) {
        *GLOBAL_CONFIG.write().await = config
    }
}
