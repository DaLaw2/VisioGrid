use std::path::{Path, PathBuf};
use tokio::fs;
use lazy_static::lazy_static;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use crate::utils::static_files::StaticFiles;
use crate::utils::logger::{Logger, LogLevel};

lazy_static! {
    static ref FILE_MANAGER: RwLock<FileManager> = RwLock::new(FileManager {});
}

pub struct FileManager;

impl FileManager {
    pub async fn initialize() {
        Logger::add_system_log(LogLevel::INFO, "File Manager: Initializing.".to_string()).await;
        let folders = ["SavedModel", "SavedFile", "Script"];
        for &folder_name in &folders {
            match fs::create_dir(folder_name).await {
                Ok(_) => Logger::add_system_log(LogLevel::INFO, format!("File Manager: Create {} folder successfully.", folder_name)).await,
                Err(err) => Logger::add_system_log(LogLevel::ERROR, format!("File Manager: Cannot create {} folder.\nReason: {}", folder_name, err)).await
            }
        }
        if let Err(err) = Self::extract_embed_folders().await {
            Logger::add_system_log(LogLevel::ERROR, err).await;
        }
        Logger::add_system_log(LogLevel::INFO, "File Manager: Initialization completed.".to_string()).await;
    }

    pub async fn cleanup() {
        Logger::add_system_log(LogLevel::INFO, "File Manager: Cleaning up.".to_string()).await;
        let folders = ["SavedModel", "SavedFile", "Script"];
        for &folder_name in &folders {
            match fs::remove_dir_all(folder_name).await {
                Ok(_) => Logger::add_system_log(LogLevel::INFO, format!("File Manager: Deleted {} folder successfully.", folder_name)).await,
                Err(err) => Logger::add_system_log(LogLevel::ERROR, format!("File Manager: Cannot delete {} folder.\nReason: {}", folder_name, err)).await
            }
        };
        Logger::add_system_log(LogLevel::INFO, "File Manager: Cleanup completed.".to_string()).await;
    }

    pub async fn extract_embed_folders() -> Result<(), String> {
        let folders = ["Script"];
        for file in StaticFiles::iter() {
            let file_path = PathBuf::from(file.as_ref());
            if let Some(first_part) = file_path.iter().next().and_then(|s| s.to_str()) {
                if folders.contains(&first_part) {
                    let relative_path = file_path.strip_prefix(first_part).unwrap_or(&file_path);
                    let full_path = PathBuf::from(first_part).join(relative_path);
                    let file_data = &StaticFiles::get(file.as_ref())
                        .ok_or("File Manager: File not in static files.")?
                        .data;
                    let mut file = File::create(full_path).await
                        .map_err(|err| format!("File Manager: Unable to create file.\nReason: {}", err))?;
                    file.write_all(file_data).await
                        .map_err(|err| format!("File Manager: Unable to write data to file.\nReason: {}", err))?;
                }
            }
        }
        Ok(())
    }
}
