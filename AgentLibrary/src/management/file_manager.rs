use crate::utils::logging::*;
use crate::utils::logging::{LogLevel, Logger};
use crate::utils::static_files::StaticFiles;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::process::Command as AsyncCommand;

pub struct FileManager;

impl FileManager {
    pub async fn initialize() {
        logging_information!(SystemEntry::Initializing);
        let folders = ["SavedModel", "SavedFile", "Result", "Script", "Script/ultralytics"];
        for &folder_name in &folders {
            let path = PathBuf::from(folder_name);
            if let Err(err) = fs::create_dir(folder_name).await {
                logging_error!(IOEntry::CreateDirectoryError(path.display(), err));
            }
        }
        if let Err(entry) = Self::clone_repository().await {
            logging_entry!(entry);
        }
        if let Err(entry) = Self::extract_embed_folders().await {
            logging_entry!(entry);
        }
        logging_information!(SystemEntry::InitializeComplete);
    }

    pub async fn cleanup() {
        logging_information!(SystemEntry::Cleaning);
        let folders = ["SavedModel", "SavedFile", "Result", "Script"];
        for &folder_name in &folders {
            let path = PathBuf::from(folder_name);
            if let Err(err) = fs::remove_dir_all(folder_name).await {
                logging_error!(IOEntry::DeleteDirectoryError(path.display(), err));
            }
        };
        logging_information!(SystemEntry::CleanComplete);
    }

    pub async fn extract_embed_folders() -> Result<(), LogEntry> {
        for file in StaticFiles::iter() {
            let file_path = PathBuf::from(file.as_ref());
            if let Some(first_part) = file_path.iter().next().and_then(|s| s.to_str()) {
                if first_part.eq("script") {
                    let relative_path = file_path.strip_prefix(first_part).unwrap_or(&file_path);
                    let full_path = PathBuf::from(format!("Script/{}", relative_path.display()));
                    let file_data = &StaticFiles::get(file.as_ref())
                        .ok_or(error_entry!("Unable to read file", format!("File: {}", full_path.display())))?
                        .data;
                    let mut file = File::create(&full_path).await
                        .map_err(|err| error_entry!("Unable to create file", format!("File: {}, Err: {}", full_path.display(), err)))?;
                    file.write_all(file_data).await
                        .map_err(|err| error_entry!("Unable to write file", format!("File: {}, Err: {}", full_path.display(), err)))?;
                }
            }
        }
        Ok(())
    }

    pub async fn clone_repository() -> Result<(), LogEntry> {
        let yolov4_repository = "https://github.com/WongKinYiu/PyTorch_YOLOv4";
        let yolov7_repository = "https://github.com/WongKinYiu/yolov7";
        #[cfg(target_os = "windows")]
            let mut cmd = AsyncCommand::new("cmd");
        #[cfg(target_os = "linux")]
            let mut cmd = AsyncCommand::new("sh");
        let mut process = cmd
            .arg(if cfg!(target_os = "windows") { "/C" } else { "-c" })
            .arg(format!("cd Script/ && git clone {} --depth 1 && git clone {} --depth 1", yolov4_repository, yolov7_repository))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| critical_entry!(SystemEntry::ChildProcessError(err.to_string())))?;
        let status = process.wait().await
            .map_err(|err| error_entry!(SystemEntry::ChildProcessError(err.to_string())))?;
        if !status.success() {
            let err = format!("Process exit with code: {}", status.code().unwrap_or(-1));
            Err(error_entry!(SystemEntry::ChildProcessError(err)))?
        }
        Ok(())
    }
}
