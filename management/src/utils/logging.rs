pub use crate::{logging_alert, logging_console, logging_critical, logging_debug, logging_emergency, logging_entry, logging_error, logging_information, logging_warning};
pub use common::utils::log_entry::gstreamer::GStreamerEntry;
pub use common::utils::log_entry::io::IOEntry;
pub use common::utils::log_entry::misc::MiscEntry;
pub use common::utils::log_entry::network::NetworkEntry;
pub use common::utils::log_entry::system::SystemEntry;
pub use common::utils::log_entry::task::TaskEntry;
pub use common::utils::logging::*;
pub use common::{alert_entry, critical_entry, debug_entry, emergency_entry, error_entry, information_entry, warning_entry};

use chrono::{DateTime, Local};
use lazy_static::lazy_static;
use std::collections::HashMap;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

lazy_static! {
    static ref LOGGER: RwLock<Logger> = RwLock::new(Logger::new());
}

pub struct Logger {
    system_log: Vec<LogEntry>,
    agent_log: HashMap<Uuid, Vec<LogEntry>>,
}

impl Logger {
    fn new() -> Self {
        let mut system_log = Vec::new();
        let log_entry = LogEntry::new(LogLevel::Information, "Logger", "Online now", "");
        system_log.push(log_entry);
        Self {
            system_log,
            agent_log: HashMap::new(),
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

    pub async fn add_agent_log<T: Into<String>, U: Into<String>, V: Into<String>>(agent_id: Uuid, level: LogLevel, position: T, message: U, debug_info: V) {
        let log_entry = LogEntry::new(level, position, message, debug_info);
        Self::logging_console(log_entry.clone());
        let mut logger = Self::instance_mut().await;
        if !logger.agent_log.contains_key(&agent_id) {
            logger.agent_log.insert(agent_id, Vec::new());
        }
        if let Some(log) = logger.agent_log.get_mut(&agent_id) {
            log.push(log_entry);
        }
    }

    pub async fn add_system_log_entry(log_entry: LogEntry) {
        Self::logging_console(log_entry.clone());
        let mut logger = Self::instance_mut().await;
        logger.system_log.push(log_entry);
    }

    pub async fn add_agent_log_entry(agent_id: Uuid, log_entry: LogEntry) {
        Self::logging_console(log_entry.clone());
        let mut logger = Self::instance_mut().await;
        if !logger.agent_log.contains_key(&agent_id) {
            logger.agent_log.insert(agent_id, Vec::new());
        }
        if let Some(log) = logger.agent_log.get_mut(&agent_id) {
            log.push(log_entry);
        }
    }

    pub fn logging_console(log_entry: LogEntry) {
        println!("{}", log_entry.to_colored_string());
    }

    pub async fn get_system_logs() -> Vec<LogEntry> {
        Self::instance().await.system_log.clone()
    }

    pub async fn get_agent_logs(agent_id: Uuid) -> Option<Vec<LogEntry>> {
        let logger = Self::instance().await;
        logger.agent_log.get(&agent_id).and_then(|entry| Some(entry.clone()))
    }

    pub async fn get_system_logs_since(time: DateTime<Local>) -> Vec<LogEntry> {
        let logger = Self::instance().await;
        let index = logger.system_log.binary_search_by(|entry| entry.timestamp.cmp(&time)).unwrap_or_else(|x| x);
        logger.system_log[index..].to_vec()
    }

    pub async fn get_agent_logs_since(agent_id: Uuid, time: DateTime<Local>) -> Option<Vec<LogEntry>> {
        let logger = Self::instance().await;
        let logs = logger.agent_log.get(&agent_id)?;
        let index = logs.binary_search_by(|entry| entry.timestamp.cmp(&time)).unwrap_or_else(|x| x);
        Some(logs[index..].to_vec())
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
    ($uuid:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Debug, format!("{}:{}", file!(), line!()), $message, $debug_info).await
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
    ($uuid:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Information, format!("{}:{}", file!(), line!()), $message, $debug_info).await
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
    ($uuid:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Warning, format!("{}:{}", file!(), line!()), $message, $debug_info).await
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
    ($uuid:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Error, format!("{}:{}", file!(), line!()), $message, $debug_info).await
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
    ($uuid:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Critical, format!("{}:{}", file!(), line!()), $message, $debug_info).await
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
    ($uuid:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Alert, format!("{}:{}", file!(), line!()), $message, $debug_info).await
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
    ($uuid:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Emergency, format!("{}:{}", file!(), line!()), $message, $debug_info).await
    };
}

#[macro_export]
macro_rules! logging_entry {
    ($entry:expr) => {
        Logger::add_system_log_entry($entry).await
    };
    ($uuid:expr, $entry:expr) => {
        Logger::add_agent_log_entry($uuid, $entry).await
    };
}

#[macro_export]
macro_rules! logging_console {
    ($entry:expr) => {
        Logger::logging_console($entry)
    };
}
