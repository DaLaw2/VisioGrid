use colored::*;
use std::fmt::Display;
use chrono::{DateTime, Local};

pub use crate::{debug_entry, information_entry, notice_entry, warning_entry, error_entry, critical_entry, alert_entry, emergency_entry, logging_console};

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

impl LogLevel {
    pub fn to_plain_string(&self) -> String {
        match self {
            LogLevel::Debug => "Debug      ".to_string(),
            LogLevel::Information => "Information".to_string(),
            LogLevel::Notice => "Notice     ".to_string(),
            LogLevel::Warning => "Warning    ".to_string(),
            LogLevel::Error => "Error      ".to_string(),
            LogLevel::Critical => "Critical   ".to_string(),
            LogLevel::Alert => "Alert      ".to_string(),
            LogLevel::Emergency => "Emergency  ".to_string(),
        }
    }

    pub fn to_colored_string(&self) -> ColoredString {
        match self {
            LogLevel::Debug => "Debug      ".to_string().bright_black(),
            LogLevel::Information => "Information".to_string().bright_blue(),
            LogLevel::Notice => "Notice     ".to_string().bright_green(),
            LogLevel::Warning => "Warning    ".to_string().yellow(),
            LogLevel::Error => "Error      ".to_string().bright_red(),
            LogLevel::Critical => "Critical   ".to_string().bright_yellow(),
            LogLevel::Alert => "Alert      ".to_string().red(),
            LogLevel::Emergency => "Emergency  ".to_string().magenta(),
        }
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = self.to_plain_string();
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
    pub fn new<T: Into<String>, U: Into<String>, V: Into<String>>(level: LogLevel, position: T, message: U, debug_info: V) -> Self {
        Self {
            level,
            timestamp: Local::now(),
            position: position.into(),
            message: message.into(),
            debug_info: debug_info.into(),
        }
    }
}

impl LogEntry {
    pub fn to_plain_string(&self) -> String {
        let level = self.level.to_plain_string();
        let timestramp = self.timestamp.format("%Y/%m/%d %H:%M:%S").to_string();
        let position = self.position.clone();
        let message = self.message.clone();
        let str = if self.debug_info.is_empty() {
            format!("[{}] {} {}: {}", level, timestramp, position, message)
        } else {
            let debug_info = self.debug_info.bright_black();
            format!("[{}] {} {}: {}\n{}", level, timestramp, position, message, debug_info)
        };
        str
    }

    pub fn to_colored_string(&self) -> String {
        let level = self.level.to_colored_string();
        let timestramp = self.timestamp.format("%Y/%m/%d %H:%M:%S").to_string();
        let position = self.position.cyan();
        let message = self.message.white();
        let str = if self.debug_info.is_empty() {
            format!("[{}] {} {}: {}", level, timestramp, position, message)
        } else {
            let debug_info = self.debug_info.bright_black();
            format!("[{}] {} {}: {}\n{}", level, timestramp, position, message, debug_info)
        };
        str
    }
}

impl Display for LogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = self.to_plain_string();
        write!(f, "{}", str)
    }
}

pub fn logging_console(log_entry: LogEntry) {
    println!("{}", log_entry.to_colored_string());
}

#[macro_export]
macro_rules! debug_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Debug, $position, $message, "")
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Debug, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info))
    };
}

#[macro_export]
macro_rules! information_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Information, $position, $message, "")
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Information, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info))
    };
}

#[macro_export]
macro_rules! notice_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Notice, $position, $message, "")
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Notice, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info))
    };
}

#[macro_export]
macro_rules! warning_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Warning, $position, $message, "")
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Warning, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info))
    };
}

#[macro_export]
macro_rules! error_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Error, $position, $message, "")
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Error, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info))
    };
}

#[macro_export]
macro_rules! critical_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Critical, $position, $message, "")
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Critical, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info))
    };
}

#[macro_export]
macro_rules! alert_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Alert, $position, $message, "")
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Alert, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info))
    };
}

#[macro_export]
macro_rules! emergency_entry {
    ($position:expr, $message:expr) => {
        LogEntry::new(LogLevel::Emergency, $position, $message, "")
    };
    ($position:expr, $message:expr, $debug_info:expr) => {
        LogEntry::new(LogLevel::Emergency, $position, $message, format!("{}:{} {}", file!(), line!(), $debug_info))
    };
}

#[macro_export]
macro_rules! logging_console {
    ($log_entry:expr) => {
        crate::utils::logging::logging_console($log_entry);
    };
}
