use tokio::fs;
use lazy_static::lazy_static;
use tokio::sync::{Mutex, MutexGuard};
use crate::utils::logger::{Logger, LogLevel};

lazy_static! {
    static ref GLOBAL_FILE_MANAGER: Mutex<FileManager> = Mutex::new(FileManager::new());
}

pub struct FileManager;

impl FileManager {
    pub fn new() -> Self {
        FileManager
    }

    pub async fn initialize() {
        match fs::create_dir("SavedModel").await {
            Ok(_) => Logger::instance().await.append_system_log(LogLevel::INFO, "Create WebSave folder success.".to_string()),
            Err(_) => Logger::instance().await.append_system_log(LogLevel::ERROR, "Fail create WebSave folder.".to_string())
        }
        match fs::create_dir("SavedFile").await {
            Ok(_) => Logger::instance().await.append_system_log(LogLevel::INFO, "Create web Unzip folder success.".to_string()),
            Err(_) => Logger::instance().await.append_system_log(LogLevel::ERROR, "Fail create Unzip folder.".to_string())
        }
    }

    pub async fn instance() -> MutexGuard<'static, FileManager> {
        GLOBAL_FILE_MANAGER.lock().await
    }

    pub async fn clean() {
        match fs::remove_dir_all("SavedModel").await {
            Ok(_) => Logger::instance().await.append_system_log(LogLevel::INFO, "Destroy WebSave folder success.".to_string()),
            Err(_) => Logger::instance().await.append_system_log(LogLevel::ERROR, "Fail destroy WebSave folder.".to_string())
        }
        match fs::remove_dir_all("SavedFile").await {
            Ok(_) => Logger::instance().await.append_system_log(LogLevel::INFO, "Destroy Unzip folder success.".to_string()),
            Err(_) => Logger::instance().await.append_system_log(LogLevel::ERROR, "Fail destroy Unzip folder.".to_string())
        }
    }
}
