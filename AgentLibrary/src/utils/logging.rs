pub use Common::utils::logging::*;
pub use Common::{debug_entry, information_entry, notice_entry, warning_entry, error_entry, critical_entry, alert_entry, emergency_entry};
pub use crate::{logging_debug, logging_information, logging_notice, logging_warning, logging_error, logging_critical, logging_alert, logging_emergency, logging_entry};

use lazy_static::lazy_static;
use std::collections::VecDeque;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

lazy_static! {
    static ref LOGGER: RwLock<Logger> = RwLock::new(Logger::new());
}

pub struct Logger {
    system_log: VecDeque<LogEntry>,
}

impl Logger {
    fn new() -> Self {
        let mut system_log = VecDeque::new();
        let log_entry = information_entry!("Logger", "Online now");
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

    pub async fn add_system_log<T: Into<String>, U: Into<String>, V: Into<String>>(level: LogLevel, position: T, message: U, debug_info: V) {
        let log_entry = LogEntry::new(level, position, message, debug_info);
        Self::logging_console(log_entry.clone());
        let mut logger = Self::instance_mut().await;
        logger.system_log.push_back(log_entry);
    }

    pub async fn add_system_log_entry(log_entry: LogEntry) {
        Self::logging_console(log_entry.clone());
        let mut logger = Self::instance_mut().await;
        logger.system_log.push_back(log_entry);
    }

    pub fn logging_console(log_entry: LogEntry) {
        println!("{}", log_entry.to_colored_string());
    }

    pub async fn get_system_logs() -> VecDeque<LogEntry> {
        Self::instance().await.system_log.clone()
    }
}

#[macro_export]
macro_rules! logging_debug {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Debug, $position, $message, "").await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Debug, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_information {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Information, $position, $message, "").await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Information, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_notice {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Notice, $position, $message, "").await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Notice, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_warning {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Warning, $position, $message, "").await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Warning, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_error {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Error, $position, $message, "").await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Error, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_critical {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Critical, $position, $message, "").await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Critical, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_alert {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Alert, $position, $message, "").await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Alert, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_emergency {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Emergency, $position, $message, "").await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Emergency, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_entry {
    ($entry:expr) => {
        Logger::add_system_log_entry($entry).await
    };
}
