pub use Common::utils::logger::*;
use chrono::Local;
use lazy_static::lazy_static;
use std::collections::VecDeque;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

lazy_static! {
    static ref GLOBAL_LOGGER: RwLock<Logger> = RwLock::new(Logger::new());
}

pub struct Logger {
    system_log: VecDeque<LogEntry>,
}

impl Logger {
    fn new() -> Self {
        let mut system_log = VecDeque::new();
        let log_entry = LogEntry::new(LogLevel::INFO, "Logger: Log enable.".to_string());
        system_log.push_back(log_entry);
        Self {
            system_log,
        }
    }

    pub async fn instance() -> RwLockReadGuard<'static, Logger> {
        GLOBAL_LOGGER.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Logger> {
        GLOBAL_LOGGER.write().await
    }

    pub async fn append_system_log(level: LogLevel, message: String) {
        let date = Local::now();
        let timestamp = date.format("%Y/%m/%d %H:%M:%S").to_string();
        println!("{}", format!("{} [{}] {}", timestamp, level, message));
        let log_entry = LogEntry::new(level, message);
        let mut logger = Self::instance_mut().await;
        logger.system_log.push_back(log_entry);
    }

    pub async fn get_system_logs() -> VecDeque<LogEntry> {
        Self::instance().await.system_log.clone()
    }
}
