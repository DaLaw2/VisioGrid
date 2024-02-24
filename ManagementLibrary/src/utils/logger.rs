pub use Common::utils::logger::*;
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
        let log_entry = LogEntry::new(LogLevel::INFO, "Logger: Log enable.".to_string());
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

    pub async fn append_system_log(level: LogLevel, message: String) {
        let date = Local::now();
        let timestamp = date.format("%Y/%m/%d %H:%M:%S").to_string();
        println!("{}", format!("{} [{}] {}", timestamp, level, message));
        let log_entry = LogEntry::new(level, message);
        let mut logger = Self::instance_mut().await;
        logger.system_log.push_back(log_entry);
    }

    pub async fn append_agent_log(agent_id: Uuid, level: LogLevel, message: String) {
        let log_entry = LogEntry::new(level, message);
        let mut logger = Self::instance_mut().await;
        if !logger.agent_log.contains_key(&agent_id) {
            logger.agent_log.insert(agent_id, VecDeque::new());
        }
        //Impossible error, because it has been checked before.
        logger.agent_log.get_mut(&agent_id).unwrap().push_back(log_entry);
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
