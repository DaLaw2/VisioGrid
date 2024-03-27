use lazy_static::lazy_static;
use std::collections::VecDeque;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub use Common::utils::logger::*;
pub use crate::{logging_info, logging_warning, logging_error, logging_entry};

lazy_static! {
    static ref LOGGER: RwLock<Logger> = RwLock::new(Logger::new());
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
        LOGGER.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Logger> {
        LOGGER.write().await
    }

    pub async fn add_system_log<T: Into<String>>(level: LogLevel, message: T) {
        let log_entry = LogEntry::new(level, message);
        println!("{log_entry}");
        let mut logger = Self::instance_mut().await;
        logger.system_log.push_back(log_entry);
    }

    pub async fn add_system_log_entry(log_entry: LogEntry) {
        let mut logger = Self::instance_mut().await;
        println!("{log_entry}");
        logger.system_log.push_back(log_entry);
    }

    pub async fn get_system_logs() -> VecDeque<LogEntry> {
        Self::instance().await.system_log.clone()
    }
}

#[macro_export]
macro_rules! logging_info {
    ($msg:expr) => {
        Logger::add_system_log(LogLevel::INFO, $msg).await;
    };
}

#[macro_export]
macro_rules! logging_warning {
    ($msg:expr) => {
        Logger::add_system_log(LogLevel::WARNING, $msg).await;
    };
}

#[macro_export]
macro_rules! logging_error {
    ($msg:expr) => {
        Logger::add_system_log(LogLevel::ERROR, $msg).await;
    };
}

#[macro_export]
macro_rules! logging_entry {
    ($entry:expr) => {
        Logger::add_system_log_entry($entry).await;
    };
}
