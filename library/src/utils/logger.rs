use uuid::Uuid;
use chrono::Local;
use tokio::sync::RwLock;
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref GLOBAL_LOGGER: RwLock<Logger> = RwLock::new(Logger::new());
}

#[derive(Copy, Clone)]
pub enum LogLevel {
    INFO,
    WARNING,
    ERROR,
}

pub struct Logger {
    system_log: String,
    node_log: HashMap<Uuid, String>,
}

impl Logger {
    fn new() -> Self {
        let date = Local::now();
        let timestamp = date.format("%Y/%m/%d %H:%M:%S").to_string();
        let log_entry = format!("{} [{}] {}", timestamp, "INFO", "Log enable.");
        Self {
            system_log: log_entry,
            node_log: HashMap::new()
        }
    }

    pub async fn append_system_log(log_level: LogLevel, message: String) {
        let mut logger = GLOBAL_LOGGER.write().await;
        let date = Local::now();
        let timestamp = date.format("%Y/%m/%d %H:%M:%S").to_string();
        let log_entry = format!("{} [{}] {}", timestamp, match log_level {
            LogLevel::INFO => "INFO",
            LogLevel::WARNING => "WARNING",
            LogLevel::ERROR => "ERROR",
        }, message);
        logger.system_log.push_str(&log_entry);
        println!("{}", log_entry);
    }

    pub async fn append_node_log(node_id: Uuid, log_level: LogLevel, message: String) {
        let mut logger = GLOBAL_LOGGER.write().await;
        let date = Local::now();
        let timestamp = date.format("%Y/%m/%d %H:%M:%S").to_string();
        if !logger.node_log.contains_key(&node_id) {
            logger.node_log.insert(node_id, String::new());
        }
        let log_entry = format!("{} [{}] {}", timestamp, match log_level {
            LogLevel::INFO => "INFO",
            LogLevel::WARNING => "WARNING",
            LogLevel::ERROR => "ERROR",
        }, message);
        //Impossible error, because it has been checked before.
        logger.node_log.get_mut(&node_id).unwrap().push_str(&log_entry);
    }

    pub async fn get_system_log() -> String {
        let logger = GLOBAL_LOGGER.read().await;
        logger.system_log.clone()
    }

    pub async fn get_node_log(node_id: Uuid) -> String {
        let logger = GLOBAL_LOGGER.read().await;
        let node_log = logger.node_log.get(&node_id);
        match node_log {
            Some(str) => str.clone(),
            //When node has not written to the log.
            None => String::new()
        }
    }
}
