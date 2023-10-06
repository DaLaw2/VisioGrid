use tokio::fs;
use crate::utils::logger::{Logger, LogLevel};
use tokio::sync::{OnceCell, Mutex, MutexGuard};

static GLOBAL_FILE_MANAGER: OnceCell<Mutex<FileManager>> = OnceCell::const_new();

struct FileManager;

impl FileManager {
    async fn new() -> FileManager {
        match fs::create_dir("WebSave").await {
            Ok(_) => Logger::instance().await.append_system_log(LogLevel::INFO, "Create WebSave folder success.".to_string()),
            Err(_) => Logger::instance().await.append_system_log(LogLevel::ERROR, "Fail create WebSave folder.".to_string())
        }
        match fs::create_dir("Unzip").await {
            Ok(_) => Logger::instance().await.append_system_log(LogLevel::INFO, "Create web Unzip folder success.".to_string()),
            Err(_) => Logger::instance().await.append_system_log(LogLevel::ERROR, "Fail create Unzip folder.".to_string())
        }
        FileManager
    }

    pub async fn instance() -> MutexGuard<'static, FileManager> {
        let mutex = GLOBAL_FILE_MANAGER.get_or_init(|| async {
            let fm = FileManager::new().await;
            Mutex::new(fm)
        }).await;
        mutex.lock().await
    }

    pub async fn clean() {
        match fs::remove_dir_all("WebSave").await {
            Ok(_) => Logger::instance().await.append_system_log(LogLevel::INFO, "Destroy WebSave folder success.".to_string()),
            Err(_) => Logger::instance().await.append_system_log(LogLevel::ERROR, "Fail destroy WebSave folder.".to_string())
        }
        match fs::remove_dir_all("Unzip").await {
            Ok(_) => Logger::instance().await.append_system_log(LogLevel::INFO, "Destroy Unzip folder success.".to_string()),
            Err(_) => Logger::instance().await.append_system_log(LogLevel::ERROR, "Fail destroy Unzip folder.".to_string())
        }
    }
}
