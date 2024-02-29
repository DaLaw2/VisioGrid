use tokio::fs;
use crate::utils::logger::{Logger, LogLevel};

pub struct FileManager;
impl FileManager {
    pub async fn initialize() {
        Logger::add_system_log(LogLevel::INFO, "File Manager: Initializing.".to_string()).await;
        let folders = ["SavedModel", "SavedFile", "Transform"];
        for &folder_name in &folders {
            match fs::create_dir(folder_name).await {
                Ok(_) => Logger::add_system_log(LogLevel::INFO, format!("File Manager: Create {} folder successfully.", folder_name)).await,
                Err(err) => Logger::add_system_log(LogLevel::ERROR, format!("File Manager: Cannot create {} folder.\nReason: {}", folder_name, err)).await
            }
        }
        Logger::add_system_log(LogLevel::INFO, "File Manager: Initialization completed.".to_string()).await;
    }

    pub async fn cleanup() {
        Logger::add_system_log(LogLevel::INFO, "File Manager: Cleaning up.".to_string()).await;
        let folders = ["SavedModel", "SavedFile", "Transform"];
        for &folder_name in &folders {
            match fs::remove_dir_all(folder_name).await {
                Ok(_) => Logger::add_system_log(LogLevel::INFO, format!("File Manager: Deleted {} folder successfully.", folder_name)).await,
                Err(err) => Logger::add_system_log(LogLevel::ERROR, format!("File Manager: Cannot delete {} folder.\nReason: {}", folder_name, err)).await
            }
        };
        Logger::add_system_log(LogLevel::INFO, "File Manager: Cleanup completed.".to_string()).await;
    }
}
