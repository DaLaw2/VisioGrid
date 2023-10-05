use tokio::fs;
use serde::Deserialize;
use lazy_static::lazy_static;
use tokio::sync::{Mutex, MutexGuard};

lazy_static! {
    static ref GLOBAL_CONFIG: Mutex<Config> = Mutex::new(Config::new());
}

enum ConfigOption {
    HttpServerBindPort,
    NodeListenPort,
    BindRetryDuration,
    DedicatedPortRange
}

#[derive(Debug, Deserialize)]
pub struct Config {
    http_server_bind_port: usize,
    node_listen_port: usize,
    bind_retry_duration: usize,
    dedicated_port_range: (usize, usize),
}

impl Config {
    async fn new() -> Self {
        let toml_string = fs::read_to_string("./Config.toml").await.expect("No configuration found.");
        let config: Self = toml::from_str(&toml_string).expect("Fail parse configuration.");
        config
    }

    pub async fn instance() -> MutexGuard<'static, Config> {
        GLOBAL_CONFIG.lock().await
    }
}
pub struct Configurator {
}

impl Configurator {
}
