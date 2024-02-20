use lazy_static::lazy_static;
use chrono::{DateTime, Local};
use std::collections::VecDeque;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

lazy_static! {
    static ref GLOBAL_LOGGER: RwLock<Logger> = RwLock::new(Logger::new());
}

pub struct Logger {
    system_log: VecDeque<LogEntry>,
}

#[derive(Copy, Clone)]
pub enum LogLevel {
    INFO,
    WARNING,
    ERROR,
}

#[derive(Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub level: LogLevel,
    pub message: String,
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
        println!("{}", format!("{} [{}] {}", timestamp, level.to_string(), message));
        let log_entry = LogEntry::new(level, message);
        let mut logger = Self::instance_mut().await;
        logger.system_log.push_back(log_entry);
    }

    pub async fn get_system_logs() -> VecDeque<LogEntry> {
        Self::instance().await.system_log.clone()
    }
}

impl ToString for LogLevel {
    fn to_string(&self) -> String {
        match self {
            LogLevel::INFO => "INFO".to_string(),
            LogLevel::WARNING => "WARNING".to_string(),
            LogLevel::ERROR => "ERROR".to_string(),
        }
    }
}

impl LogEntry {
    fn new(level: LogLevel, message: String) -> Self {
        Self {
            timestamp: Local::now(),
            level,
            message,
        }
    }
}

impl ToString for LogEntry {
    fn to_string(&self) -> String {
        format!("{} [{}] {}", self.timestamp.format("%Y/%m/%d %H:%M:%S").to_string(), self.level.to_string(), self.message)
    }
}
