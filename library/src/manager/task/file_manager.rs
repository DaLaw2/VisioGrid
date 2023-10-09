use tokio::fs;
use std::sync::Arc;
use std::ffi::OsStr;
use tokio::sync::Mutex;
use tokio::time::sleep;
use std::time::Duration;
use lazy_static::lazy_static;
use std::path::{Path, PathBuf};
use std::collections::VecDeque;
use crate::manager::task::definition::Task;
use crate::utils::logger::{Logger, LogLevel};

lazy_static! {
    static ref GLOBAL_FILE_MANAGER: Arc<Mutex<FileManager>> = Arc::new(Mutex::new(FileManager::new()));
}

pub struct FileManager {
    task_queue: VecDeque<Task>
}

impl FileManager {
    fn new() -> Self {
        FileManager {
            task_queue: VecDeque::new()
        }
    }

    pub async fn initialize() {
        let folders = ["SavedModel", "SavedFile", "PreProcess", "PostProcess", "Result"];
        for &folder_name in &folders {
            match fs::create_dir(folder_name).await {
                Ok(_) => Logger::instance().await.append_system_log(LogLevel::INFO, format!("Create {} folder success.", folder_name)),
                Err(_) => Logger::instance().await.append_system_log(LogLevel::ERROR, format!("Fail create {} folder.", folder_name))
            }
        }
    }

    pub async fn cleanup() {
        let folders = ["SavedModel", "SavedFile", "PreProcess", "PostProcess", "Result"];
        for &folder_name in &folders {
            match fs::remove_dir_all(folder_name).await {
                Ok(_) => Logger::instance().await.append_system_log(LogLevel::INFO, format!("Destroy {} folder success.", folder_name)),
                Err(_) => Logger::instance().await.append_system_log(LogLevel::ERROR, format!("Fail destroy {} folder.", folder_name))
            }
        };
    }

    pub async fn run() {
        let file_manager = GLOBAL_FILE_MANAGER.clone();
        tokio::spawn(async move {
            loop {
                let task = {
                    let mut file_manager = file_manager.lock().await;
                    file_manager.task_queue.pop_front()
                };
                match task {
                    Some(task) => {
                        match Path::new(&task.inference_filename).extension().and_then(OsStr::to_str) {
                            Some("jpg") | Some("jpeg") => {
                                let source_path: PathBuf = format!("./SavedFile/{}", task.inference_filename).into();
                                let destination_path: PathBuf = format!("./PreProcess/{}", task.inference_filename).into();
                                match fs::rename(source_path, destination_path).await {
                                    Ok(_) => Self::next_step(task).await,
                                    Err(_) => Logger::instance().await.append_global_log(LogLevel::ERROR, format!("The task of IP:{} failed.", task.ip))
                                }
                            },
                            Some("gif") | Some("mp4") | Some("wav") | Some("avi") | Some("mkv") => Self::extract_media(task).await,
                            Some("zip") => Self::extract_zip(task).await,
                            _ => Logger::instance().await.append_global_log(LogLevel::INFO, format!("The task of IP:{} failed.", task.ip)),
                        }
                    },
                    None => sleep(Duration::from_millis(100)).await
                }
            }
        });
    }

    pub async fn add_task(task: Task) {
        let mut manager = GLOBAL_FILE_MANAGER.lock().await;
        manager.task_queue.push_back(task);
    }

    async fn extract_media(task: Task) {
        println!("Call extract_media function.")
    }

    async fn extract_zip(task: Task) {
        println!("Call extract_zip function.")
    }

    async fn next_step(task: Task) {
        println!("Call next_step function.")
    }
}
