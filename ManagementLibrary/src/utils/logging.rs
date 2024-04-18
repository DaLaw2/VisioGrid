pub use Common::utils::logging::*;
pub use Common::{debug_entry, information_entry, notice_entry, warning_entry, error_entry, critical_entry, alert_entry, emergency_entry};
pub use crate::{logging_debug, logging_information, logging_notice, logging_warning, logging_error, logging_critical, logging_alert, logging_emergency, logging_entry, logging_console};

use uuid::Uuid;
use lazy_static::lazy_static;
use chrono::{DateTime, Local};
use std::collections::{HashMap, VecDeque};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

lazy_static! {
    static ref LOGGER: RwLock<Logger> = RwLock::new(Logger::new());
}

pub struct Logger {
    system_log: VecDeque<LogEntry>,
    agent_log: HashMap<Uuid, VecDeque<LogEntry>>,
}

impl Logger {
    fn new() -> Self {
        let mut system_log = VecDeque::new();
        let log_entry = LogEntry::new(LogLevel::Information, "Logger", "Online now", "");
        system_log.push_back(log_entry);
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
        logger.system_log.push_back(log_entry);
    }

    pub async fn add_agent_log<T: Into<String>, U: Into<String>, V: Into<String>>(agent_id: Uuid, level: LogLevel, position: T, message: U, debug_info: V) {
        let log_entry = LogEntry::new(level, position, message, debug_info);
        let mut logger = Self::instance_mut().await;
        Self::logging_console(log_entry.clone());
        if !logger.agent_log.contains_key(&agent_id) {
            logger.agent_log.insert(agent_id, VecDeque::new());
        }
        if let Some(log) = logger.agent_log.get_mut(&agent_id) {
            log.push_back(log_entry);
        }
    }

    pub async fn add_system_log_entry(log_entry: LogEntry) {
        Self::logging_console(log_entry.clone());
        let mut logger = Self::instance_mut().await;
        logger.system_log.push_back(log_entry);
    }

    pub async fn add_agent_log_entry(agent_id: Uuid, log_entry: LogEntry) {
        let mut logger = Self::instance_mut().await;
        if !logger.agent_log.contains_key(&agent_id) {
            logger.agent_log.insert(agent_id, VecDeque::new());
        }
        if let Some(log) = logger.agent_log.get_mut(&agent_id) {
            log.push_back(log_entry);
        }
    }

    pub fn logging_console(log_entry: LogEntry) {
        println!("{log_entry}");
    }

    pub async fn get_system_logs() -> VecDeque<LogEntry> {
        Self::instance().await.system_log.clone()
    }

    pub async fn get_agent_logs(agent_id: Uuid) -> Option<VecDeque<LogEntry>> {
        let logger = Self::instance_mut().await;
        logger.agent_log.get(&agent_id).and_then(|entry| Some(entry.clone()))
    }

    pub async fn get_system_logs_since(time: DateTime<Local>) -> VecDeque<LogEntry> {
        let logger = Self::instance().await;
        logger.system_log.iter().filter(|entry| entry.timestamp > time).cloned().collect()
    }

    pub async fn get_agent_logs_since(agent_id: Uuid, time: DateTime<Local>) -> Option<VecDeque<LogEntry>> {
        let logger = Self::instance().await;
        let logs = logger.agent_log.get(&agent_id)?;
        let filter_logs = logs.iter().filter(|entry| entry.timestamp > time).cloned().collect();
        Some(filter_logs)
    }

    pub fn format_logs(logs: &VecDeque<LogEntry>) -> String {
        logs.iter().map(LogEntry::to_string).collect::<Vec<_>>().join("\n")
    }
}

#[macro_export]
macro_rules! logging_debug {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Debug, $position, $message, format!("{}:{}", file!(), line!())).await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Debug, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
    ($uuid:expr, $position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Debug, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_information {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Information, $position, $message, format!("{}:{}", file!(), line!())).await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Information, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
    ($uuid:expr, $position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Information, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_notice {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Notice, $position, $message, format!("{}:{}", file!(), line!())).await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Notice, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
    ($uuid:expr, $position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Notice, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_warning {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Warning, $position, $message, format!("{}:{}", file!(), line!())).await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Warning, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
    ($uuid:expr, $position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Warning, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_error {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Error, $position, $message, format!("{}:{}", file!(), line!())).await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Error, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
    ($uuid:expr, $position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Error, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_critical {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Critical, $position, $message, format!("{}:{}", file!(), line!())).await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Critical, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
    ($uuid:expr, $position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Critical, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_alert {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Alert, $position, $message, format!("{}:{}", file!(), line!())).await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Alert, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
    ($uuid:expr, $position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Alert, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
}

#[macro_export]
macro_rules! logging_emergency {
    ($position:expr, $message:expr) => {
        Logger::add_system_log(LogLevel::Emergency, $position, $message, format!("{}:{}", file!(), line!())).await
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_system_log(LogLevel::Emergency, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
    };
    ($uuid:expr, $position:expr, $message:expr, $debug_info:expr) => {
        Logger::add_agent_log($uuid, LogLevel::Emergency, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info)).await
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
