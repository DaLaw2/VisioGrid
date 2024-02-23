use std::fmt::Display;
use chrono::{DateTime, Local};

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

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            LogLevel::INFO => "INFO".to_string(),
            LogLevel::WARNING => "WARNING".to_string(),
            LogLevel::ERROR => "ERROR".to_string(),
        };
        write!(f, "{}", str)
    }
}

impl LogEntry {
    pub fn new(level: LogLevel, message: String) -> Self {
        Self {
            timestamp: Local::now(),
            level,
            message,
        }
    }
}

impl Display for LogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = format!("{} [{}] {}", self.timestamp.format("%Y/%m/%d %H:%M:%S").to_string(), self.level.to_string(), self.message);
        write!(f, "{}", str)
    }
}
