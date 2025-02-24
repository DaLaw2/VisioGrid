pub use crate::{logging_alert, logging_critical, logging_debug, logging_emergency, logging_entry, logging_error, logging_information, logging_warning};
pub use common::utils::log_entry::io::IOEntry;
pub use common::utils::log_entry::misc::MiscEntry;
pub use common::utils::log_entry::network::NetworkEntry;
pub use common::utils::log_entry::system::SystemEntry;
pub use common::utils::log_entry::task::TaskEntry;
pub use common::utils::logging::*;
pub use common::{alert_entry, critical_entry, debug_entry, emergency_entry, error_entry, information_entry, warning_entry};

use lazy_static::lazy_static;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

lazy_static! {
    static ref LOGGER: RwLock<Logger> = RwLock::new(Logger::new());
}

pub struct Logger {
    system_log: Vec<LogEntry>,
}

impl Logger {
    fn new() -> Self {
        let mut system_log = Vec::new();
        let log_entry = information_entry!("Logger", "Online now");
        system_log.push(log_entry);
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
        logger.system_log.push(log_entry);
    }

    pub async fn add_system_log_entry(log_entry: LogEntry) {
        Self::logging_console(log_entry.clone());
        let mut logger = Self::instance_mut().await;
        logger.system_log.push(log_entry);
    }

    pub fn logging_console(log_entry: LogEntry) {
        println!("{}", log_entry.to_colored_string());
    }

    pub async fn get_system_logs() -> Vec<LogEntry> {
        Self::instance().await.system_log.clone()
    }
}

#[macro_export]
macro_rules! logging_debug {
    ($message:expr) => {
        Logger::add_system_log(LogLevel::Debug, format!("{}:{}", file!(), line!()), $message, "").await
    };
    ($message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Debug, format!("{}:{}", file!(), line!()), $message, $debug_info).await
    };
}

#[macro_export]
macro_rules! logging_information {
    ($message:expr) => {
        Logger::add_system_log(LogLevel::Information, format!("{}:{}", file!(), line!()), $message, "").await
    };
    ($message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Information, format!("{}:{}", file!(), line!()), $message, $debug_info).await
    };
}

#[macro_export]
macro_rules! logging_warning {
    ($message:expr) => {
        Logger::add_system_log(LogLevel::Warning, format!("{}:{}", file!(), line!()), $message, "").await
    };
    ($message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Warning, format!("{}:{}", file!(), line!()), $message, $debug_info).await
    };
}

#[macro_export]
macro_rules! logging_error {
    ($message:expr) => {
        Logger::add_system_log(LogLevel::Error, format!("{}:{}", file!(), line!()), $message, "").await
    };
    ($message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Error, format!("{}:{}", file!(), line!()), $message, $debug_info).await
    };
}

#[macro_export]
macro_rules! logging_critical {
    ($message:expr) => {
        Logger::add_system_log(LogLevel::Critical, format!("{}:{}", file!(), line!()), $message, "").await
    };
    ($message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Critical, format!("{}:{}", file!(), line!()), $message, $debug_info).await
    };
}

#[macro_export]
macro_rules! logging_alert {
    ($message:expr) => {
        Logger::add_system_log(LogLevel::Alert, format!("{}:{}", file!(), line!()), $message, "").await
    };
    ($message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Alert, format!("{}:{}", file!(), line!()), $message, $debug_info).await
    };
}

#[macro_export]
macro_rules! logging_emergency {
    ($message:expr) => {
        Logger::add_system_log(LogLevel::Emergency, format!("{}:{}", file!(), line!()), $message, "").await
    };
    ($message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Emergency, format!("{}:{}", file!(), line!()), $message, $debug_info).await
    };
}

#[macro_export]
macro_rules! logging_entry {
    ($entry:expr) => {
        Logger::add_system_log_entry($entry).await
    };
}
