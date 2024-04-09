use colored::*;
use std::fmt::Display;
use chrono::{DateTime, Local};

pub use crate::{debug_entry, information_entry, notice_entry, warning_entry, error_entry, critical_entry, alert_entry, emergency_entry};

#[derive(Copy, Clone)]
pub enum LogLevel {
    Debug,
    Information,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            LogLevel::Debug => "Debug      ".to_string().bright_black(),
            LogLevel::Information => "Information".to_string().bright_blue(),
            LogLevel::Notice => "Notice     ".to_string().bright_green(),
            LogLevel::Warning => "Warning    ".to_string().yellow(),
            LogLevel::Error => "Error      ".to_string().bright_red(),
            LogLevel::Critical => "Critical   ".to_string().bright_yellow(),
            LogLevel::Alert => "Alert      ".to_string().red(),
            LogLevel::Emergency => "Emergency  ".to_string().magenta(),
        };
        write!(f, "{}", str)
    }
}

#[derive(Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub timestamp: DateTime<Local>,
    pub position: String,
    pub message: String,
    pub debug_info: String,
}

impl LogEntry {
    pub fn new<T: Into<String>>(level: LogLevel, position: T, message: T, debug_info: T) -> Self {
        Self {
            level,
            timestamp: Local::now(),
            position: position.into(),
            message: message.into(),
            debug_info: debug_info.into(),
        }
    }
}

impl Display for LogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = self.level.to_string();
        let timestramp = self.timestamp.format("%Y/%m/%d %H:%M:%S").to_string();
        let position = self.position.cyan();
        let message = self.message.white();
        let str = if self.debug_info.is_empty() {
            format!("[{}] {} {}: {}", level, timestramp, position, message)
        } else {
            let debug_info = self.debug_info.bright_black();
            format!("[{}] {} {}: {}\n{}", level, timestramp, position, message, debug_info)
        };
        write!(f, "{}", str)
    }
}

#[macro_export]
macro_rules! debug_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Debug, $position, $message, format!("{}:{}", file!(), line!()))
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Debug, $position, $message, $debug_info)
    };
}

#[macro_export]
macro_rules! information_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Information, $position, $message, format!("{}:{}", file!(), line!()))
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Information, $position, $message, $debug_info)
    };
}

#[macro_export]
macro_rules! notice_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Notice, $position, $message, format!("{}:{}", file!(), line!()))
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Notice, $position, $message, $debug_info)
    };
}

#[macro_export]
macro_rules! warning_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Warning, $position, $message, format!("{}:{}", file!(), line!()))
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Warning, $position, $message, $debug_info)
    };
}

#[macro_export]
macro_rules! error_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Error, $position, $message, format!("{}:{}", file!(), line!()))
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Error, $position, $message, $debug_info)
    };
}

#[macro_export]
macro_rules! critical_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Critical, $position, $message, format!("{}:{}", file!(), line!()))
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Critical, $position, $message, $debug_info)
    };
}

#[macro_export]
macro_rules! alert_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Alert, $position, $message, format!("{}:{}", file!(), line!()))
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Alert, $position, $message, $debug_info)
    };
}

#[macro_export]
macro_rules! emergency_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Emergency, $position, $message, format!("{}:{}", file!(), line!()))
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Emergency, $position, $message, $debug_info)
    };
}
