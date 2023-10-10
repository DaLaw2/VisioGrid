use tokio::fs;
use std::sync::Arc;
use std::ffi::OsStr;
use tokio::sync::Mutex;
use tokio::time::sleep;
use std::time::Duration;
use gstreamer::prelude::*;
use lazy_static::lazy_static;
use std::path::{Path, PathBuf};
use std::collections::VecDeque;
use crate::manager::task::definition::Task;
use crate::utils::logger::{Logger, LogLevel};

lazy_static! {
    static ref GLOBAL_FILE_MANAGER: Arc<Mutex<FileManager>> = Arc::new(Mutex::new(FileManager::new()));
}

pub struct FileManager {
    preprocessing_queue: VecDeque<Task>,
    postprocessing_queue: VecDeque<Task>
}

impl FileManager {
    fn new() -> Self {
        FileManager {
            preprocessing_queue: VecDeque::new(),
            postprocessing_queue: VecDeque::new()
        }
    }

    pub async fn initialize() {
        let folders = ["SavedModel", "SavedFile", "PreProcessing", "Processing", "PostProcessing", "Result"];
        for &folder_name in &folders {
            match fs::create_dir(folder_name).await {
                Ok(_) => Logger::instance().await.append_system_log(LogLevel::INFO, format!("Create {} folder success.", folder_name)),
                Err(_) => Logger::instance().await.append_system_log(LogLevel::ERROR, format!("Fail create {} folder.", folder_name))
            }
        }
        if let Err(err) = gstreamer::init() {
            Logger::instance().await.append_system_log(LogLevel::ERROR, format!("GStreamer initialization failed: {:?}.", err));
        } else {
            Logger::instance().await.append_system_log(LogLevel::INFO, "GStreamer initialization successful.".to_string());
        }
    }

    pub async fn cleanup() {
        let folders = ["SavedModel", "SavedFile", "PreProcessing", "Processing", "PostProcessing", "Result"];
        for &folder_name in &folders {
            match fs::remove_dir_all(folder_name).await {
                Ok(_) => Logger::instance().await.append_system_log(LogLevel::INFO, format!("Destroy {} folder success.", folder_name)),
                Err(_) => Logger::instance().await.append_system_log(LogLevel::ERROR, format!("Fail destroy {} folder.", folder_name))
            }
        };
    }

    pub async fn add_task(task: Task) {
        let mut manager = GLOBAL_FILE_MANAGER.lock().await;
        manager.preprocessing_queue.push_back(task);
    }

    pub async fn run() {
        tokio::spawn(async {
            Self::preprocessing().await
        });
        tokio::spawn(async {
            Self::postprocessing().await
        });
    }

    async fn preprocessing() {
        let file_manager = GLOBAL_FILE_MANAGER.clone();
        loop {
            let task = {
                let mut file_manager = file_manager.lock().await;
                file_manager.preprocessing_queue.pop_front()
            };
            match task {
                Some(task) => {
                    match Path::new(&task.inference_filename).extension().and_then(OsStr::to_str) {
                        Some("png") | Some("jpg") | Some("jpeg") => {
                            let source_path: PathBuf = format!("./SavedFile/{}", task.inference_filename).into();
                            let destination_path: PathBuf = format!("./PreProcessing/{}", task.inference_filename).into();
                            match fs::rename(source_path, destination_path).await {
                                Ok(_) => Self::next_step(task).await,
                                Err(_) => Logger::instance().await.append_global_log(LogLevel::ERROR, format!("The task of IP:{} failed: Fail move inference file.", task.ip))
                            }
                        },
                        Some("gif") | Some("mp4") | Some("wav") | Some("avi") | Some("mkv") => Self::extract_media(task).await,
                        Some("zip") => Self::extract_zip(task).await,
                        _ => Logger::instance().await.append_global_log(LogLevel::INFO, format!("The task of IP:{} failed: Unsupported file extension.", task.ip)),
                    }
                },
                None => sleep(Duration::from_millis(100)).await
            }
        }
    }

    async fn postprocessing() {
    }

    async fn extract_media(task: Task) {
        let source_path: PathBuf = format!("./SavedFile/{}", &task.inference_filename).into();
        let destination_folder: PathBuf = Path::new(&format!("./PreProcessing/{}", &task.inference_filename)).with_extension("").to_path_buf();
        if let Err(e) = fs::create_dir(&destination_folder).await {
            Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Failed to create directory {}: {:?}", destination_folder.display(), e));
            return;
        }
        let destination_path = destination_folder.join(task.inference_filename.clone());
        if let Err(e) = fs::rename(&source_path, &destination_path).await {
            Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Failed to move file from {} to {}: {:?}", source_path.display(), destination_path.display(), e));
            return;
        }
        let result = tokio::task::spawn_blocking(move || {
            Self::gstreamer_process(destination_path)
        }).await;
        match result {
            Ok(Ok(_)) => {
                Self::next_step(task).await;
            },
            Ok(Err(err)) => {
                Logger::instance().await.append_global_log(LogLevel::ERROR, format!("GStreamer extraction failed: {}.", err));
            },
            Err(err) => {
                Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Task panicked: {:?}.", err));
            }
        }
    }

    fn gstreamer_process(source_media: PathBuf) -> Result<(), String> {
        let pipeline_string = format!("filesrc location=\"{}\" ! decodebin ! videoconvert ! pngenc ! multifilesink location=\"{}\"", source_media.to_string_lossy(), source_media.join("%04d.png").to_string_lossy());
        let pipeline = match gstreamer::parse_launch(&pipeline_string) {
            Ok(pipeline) => pipeline,
            Err(err) => return Err(format!("Failed to parse pipeline: {:?}", err))
        };
        let bus = match pipeline.bus() {
            Some(bus) => bus,
            None => {
                return Err("Failed to get pipeline bus.".to_string());
            }
        };
        if let Err(err) = pipeline.set_state(gstreamer::State::Playing) {
            return Err(format!("Failed to set pipeline to playing: {:?}", err));
        }
        for msg in bus.iter_timed(gstreamer::ClockTime::NONE) {
            use gstreamer::MessageView;

            match msg.view() {
                MessageView::Eos(..) => break,
                MessageView::Error(err) => {
                    return if let Some(src) = msg.src() {
                        let path = src.path_string();
                        Err(format!("Error from {}: {}", path, err.error()))
                    } else {
                        Err("Error from an unknown source.".to_string())
                    }
                },
                _ => (),
            }
        }
        if let Err(err) = pipeline.set_state(gstreamer::State::Null) {
            return Err(format!("Failed to set pipeline to null: {:?}", err));
        }
        Ok(())
    }

    async fn extract_zip(_task: Task) {
        println!("Call extract_zip function.")
    }

    async fn processing(_task: Task) {
        println!("Call processing function.")
    }
}
