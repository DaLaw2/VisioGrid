use tokio::fs;
use tokio::fs::File;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tokio::io::AsyncWriteExt;
use lazy_static::lazy_static;
use tokio::process::Command as AsyncCommand;
use crate::utils::logger::*;
use crate::utils::static_files::StaticFiles;
use crate::utils::logger::{Logger, LogLevel};

lazy_static! {
    static ref FILE_MANAGER: RwLock<FileManager> = RwLock::new(FileManager {});
}

pub struct FileManager;

impl FileManager {
    pub async fn initialize() {
        logging_info!("File Manager: Initializing.");
        let folders = ["SavedModel", "SavedFile", "Script"];
        for &folder_name in &folders {
            match fs::create_dir(folder_name).await {
                Ok(_) => logging_info!(format!("File Manager: Create {} folder successfully.", folder_name)),
                Err(err) => logging_error!(format!("File Manager: Cannot create {} folder.\nReason: {}", folder_name, err)),
            }
        }
        if let Err(err) = Self::clone_repository().await {
            logging_error!(err);
        }
        if let Err(err) = Self::extract_embed_folders().await {
            logging_error!(err);
        }
        logging_info!("File Manager: Initialization completed.");
    }

    pub async fn cleanup() {
        logging_info!("File Manager: Cleaning up.");
        let folders = ["SavedModel", "SavedFile", "Script"];
        for &folder_name in &folders {
            match fs::remove_dir_all(folder_name).await {
                Ok(_) => logging_info!(format!("File Manager: Deleted {} folder successfully.", folder_name)),
                Err(err) => logging_error!(format!("File Manager: Cannot delete {} folder.\nReason: {}", folder_name, err)),
            }
        };
        logging_info!("File Manager: Cleanup completed.");
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

    pub async fn clone_repository() -> Result<(), String> {
        let yolov4_repository = "https://github.com/WongKinYiu/PyTorch_YOLOv4";
        let yolov7_repository = "https://github.com/WongKinYiu/yolov7";
        #[cfg(target_os = "windows")]
            let mut status = AsyncCommand::new("cmd")
            .arg("/C")
            .arg(format!("cd Script/ && git clone {} && git clone {}", yolov4_repository, yolov7_repository))
            .status()
            .await
            .map_err(|err| format!("File Manager: Fail to clone repository.\nReason: {}", err))?;
        #[cfg(target_os = "linux")]
        let mut status = AsyncCommand::new("sh")
            .arg("-c")
            .arg(format!("cd Script/ && git clone {} && git clone {}", yolov4_repository, yolov7_repository))
            .status()
            .await
            .map_err(|err| format!("File Manager: Fail to clone repository.\nReason: {}", err))?;
        if !status.success() {
            Err("File Manager: An error occur in clone repository.".to_string())?
        }
        Ok(())
    }
}
