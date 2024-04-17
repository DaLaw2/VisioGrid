use tokio::fs;
use tokio::fs::File;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tokio::io::AsyncWriteExt;
use lazy_static::lazy_static;
use tokio::process::Command as AsyncCommand;
use crate::utils::logging::*;
use crate::utils::static_files::StaticFiles;
use crate::utils::logging::{Logger, LogLevel};

lazy_static! {
    static ref FILE_MANAGER: RwLock<FileManager> = RwLock::new(FileManager {});
}

pub struct FileManager;

impl FileManager {
    pub async fn initialize() {
        logging_information!("File Manager", "Initializing");
        let folders = ["SavedModel", "SavedFile", "Script"];
        for &folder_name in &folders {
            match fs::create_dir(folder_name).await {
                Ok(_) => logging_information!("File Manager", format!("Create {} folder successfully", folder_name)),
                Err(err) => logging_error!("File Manager", format!("Cannot create {} folder", folder_name), format!("Err: {err}")),
            }
        }
        if let Err(entry) = Self::clone_repository().await {
            logging_entry!(entry);
        }
        if let Err(entry) = Self::extract_embed_folders().await {
            logging_entry!(entry);
        }
        logging_information!("File Manager", "Initialization completed");
    }

    pub async fn cleanup() {
        logging_information!("File Manager", "Cleaning up");
        let folders = ["SavedModel", "SavedFile", "Script"];
        for &folder_name in &folders {
            match fs::remove_dir_all(folder_name).await {
                Ok(_) => logging_information!("File Manager", format!("Delete {folder_name} folder successfully")),
                Err(err) => logging_error!("File Manager", format!("Failed to delete {folder_name} folder"), format!("Err: {err}")),
            }
        };
        logging_information!("File Manager", "Cleanup completed");
    }

    pub async fn extract_embed_folders() -> Result<(), LogEntry> {
        let folders = ["Script"];
        for file in StaticFiles::iter() {
            let file_path = PathBuf::from(file.as_ref());
            if let Some(first_part) = file_path.iter().next().and_then(|s| s.to_str()) {
                if folders.contains(&first_part) {
                    let relative_path = file_path.strip_prefix(first_part).unwrap_or(&file_path);
                    let full_path = PathBuf::from(first_part).join(relative_path);
                    let file_data = &StaticFiles::get(file.as_ref())
                        .ok_or(error_entry!("File Manager", "Unable to read file", format!("File: {}", full_path.display())))?
                        .data;
                    let mut file = File::create(&full_path).await
                        .map_err(|err| error_entry!("File Manager", "Unable to create file", format!("File: {}, Err: {}", full_path.display(), err)))?;
                    file.write_all(file_data).await
                        .map_err(|err| error_entry!("File Manager", "Unable to write file", format!("File: {}, Err: {}", full_path.display(), err)))?;
                }
            }
        }
        Ok(())
    }

    pub async fn clone_repository() -> Result<(), LogEntry> {
        let yolov4_repository = "https://github.com/WongKinYiu/PyTorch_YOLOv4";
        let yolov7_repository = "https://github.com/WongKinYiu/yolov7";
        #[cfg(target_os = "windows")]
            let status = AsyncCommand::new("cmd")
            .arg("/C")
            .arg(format!("cd Script/ && git clone {} && git clone {}", yolov4_repository, yolov7_repository))
            .status()
            .await
            .map_err(|err| error_entry!("File Manager", "Unable to create process", format!("Err: {err}")))?;
        #[cfg(target_os = "linux")]
        let status = AsyncCommand::new("sh")
            .arg("-c")
            .arg(format!("cd Script/ && git clone {} && git clone {}", yolov4_repository, yolov7_repository))
            .status()
            .await
            .map_err(|err| error_entry!("File Manager", "Unable to create process", format!("Err: {err}")))?;
        if !status.success() {
            Err(error_entry!("File Manager", "An error occurred during process execution"))?
        }
        Ok(())
    }
}
