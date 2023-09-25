use chrono::Local;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

lazy_static! {
    static ref GLOBAL_LOGGER: Mutex<Logger> = Mutex::new(Logger::new());

}

#[derive(Copy, Clone)]
pub enum LogLevel {
    INFO,
    WARNING,
    ERROR,
}

pub struct Logger {
    empty_log: String,
    system_log: String,
    global_log: String,
    node_log: HashMap<usize, String>,
}

impl Logger {
    fn new() -> Logger {
        Logger {
            empty_log: String::new(),
            system_log: String::new(),
            global_log: String::new(),
            node_log: HashMap::new()
        }
    }

    pub fn instance() -> MutexGuard<'static, Logger> {
        GLOBAL_LOGGER.lock().unwrap()
    }

    pub fn append_system_log(&mut self, log_level: LogLevel, message: String) {
        let date = Local::now();
        let timestamp = date.format("%Y/%m/%d %H:%M:%S").to_string();
        let log_entry = format!("{} [{}] {}\n", timestamp, match log_level {
            LogLevel::INFO => "INFO",
            LogLevel::WARNING => "WARNING",
            LogLevel::ERROR => "ERROR",
        }, message);
        self.system_log.push_str(&log_entry);
    }

    pub fn append_global_log(&mut self, log_level: LogLevel, message: String) {
        let date = Local::now();
        let timestamp = date.format("%Y/%m/%d %H:%M:%S").to_string();
        let log_entry = format!("{} [{}] {}\n", timestamp, match log_level {
            LogLevel::INFO => "INFO",
            LogLevel::WARNING => "WARNING",
            LogLevel::ERROR => "ERROR",
        }, message);
        self.global_log.push_str(&log_entry);
    }

    pub fn append_node_log(&mut self, node_id: usize, log_level: LogLevel, message: String) {
        let date = Local::now();
        let timestamp = date.format("%Y/%m/%d %H:%M:%S").to_string();
        if !self.node_log.contains_key(&node_id) {
            self.node_log.insert(node_id, String::new());
        }
        let log_entry = format!("{} [{}] {}\n", timestamp, match log_level {
            LogLevel::INFO => "INFO",
            LogLevel::WARNING => "WARNING",
            LogLevel::ERROR => "ERROR",
        }, message);
        //Impossible error, because it has been checked before.
        self.node_log.get_mut(&node_id).unwrap().push_str(&log_entry);
    }

    pub fn get_system_log(&self) -> &String {
        &self.system_log
    }

    pub fn get_global_log(&self) -> &String {
        &self.global_log
    }

    pub fn get_node_log(&self, node_id: usize) -> &String {
        let node_log = self.node_log.get(&node_id);
        match node_log {
            Some(str) => str,
            //When node has not written to the log.
            None => &self.empty_log
        }
    }
}