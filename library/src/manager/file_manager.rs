use tokio::fs;
use std::fs::File;
use std::io::Error;
use std::ffi::OsStr;
use tokio::sync::Mutex;
use tokio::time::sleep;
use std::time::Duration;
use gstreamer::prelude::*;
use zip::read::ZipArchive;
use lazy_static::lazy_static;
use std::path::{Path, PathBuf};
use std::collections::VecDeque;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::task_manager::TaskManager;
use crate::manager::utils::task::{Task, TaskStatus};

lazy_static! {
    static ref GLOBAL_FILE_MANAGER: Mutex<FileManager> = Mutex::new(FileManager::new());
}

pub struct FileManager {
    preprocessing_queue: VecDeque<Task>,
    postprocessing_queue: VecDeque<Task>
}

impl FileManager {
    fn new() -> Self {
        Self {
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
        loop {
            let task = {
                let mut file_manager = GLOBAL_FILE_MANAGER.lock().await;
                file_manager.preprocessing_queue.pop_front()
            };
            match task {
                Some(mut task) => {
                    match Path::new(&task.image_filename).extension().and_then(OsStr::to_str) {
                        Some("png") | Some("jpg") | Some("jpeg") => {
                            let source_path: PathBuf = Path::new(".").join("SavedFile").join(&task.image_filename);
                            let destination_path: PathBuf = Path::new(".").join("PreProcessing").join(&task.image_filename);
                            match fs::rename(source_path, destination_path).await {
                                Ok(_) => {
                                    task.unprocessed = 1;
                                    Self::task_manager_process(task).await
                                },
                                Err(_) => Logger::instance().await.append_global_log(LogLevel::ERROR, format!("The task of IP:{} failed: Fail move image file.", task.ip))
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

    async fn extract_media(mut task: Task) {
        let source_path: PathBuf = Path::new(".").join("SavedFile").join(&task.image_filename);
        let destination_path: PathBuf = Path::new(".").join("PreProcessing").join(&task.image_filename);
        let create_folder: PathBuf = destination_path.clone().with_extension("");
        if let Err(err) = fs::create_dir(&create_folder).await {
            Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Failed to create directory {}: {:?}", create_folder.display(), err));
            return;
        }
        if let Err(err) = fs::rename(&source_path, &destination_path).await {
            Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Failed to move file from {} to {}: {:?}", source_path.display(), destination_path.display(), err));
            return;
        }
        let media_path = destination_path.clone();
        let result = tokio::task::spawn_blocking(move || {
            Self::media_process(media_path)
        }).await;
        match result {
            Ok(Ok(_)) => {
                match Self::file_count(create_folder).await {
                    Ok(count) => {
                        task.unprocessed = count;
                        Self::task_manager_process(task).await;
                    }
                    Err(err) => Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Error reading directory: {}.", err))
                }
            },
            Ok(Err(err)) => Logger::instance().await.append_global_log(LogLevel::ERROR, format!("GStreamer extraction failed: {}.", err)),
            Err(err) => Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Task panicked: {:?}.", err)),
        }
    }

    fn media_process(media_path: PathBuf) -> Result<(), String> {
        let saved_path = media_path.clone().with_extension("").to_path_buf();
        let pipeline_string = format!("filesrc location={:?} ! decodebin ! videoconvert ! pngenc ! multifilesink location={:?}", media_path, saved_path.join("%d.png"));
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
        for message in bus.iter_timed(gstreamer::ClockTime::NONE) {
            use gstreamer::MessageView;

            match message.view() {
                MessageView::Eos(..) => break,
                MessageView::Error(err) => {
                    return if let Some(src) = message.src() {
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

    async fn extract_zip(mut task: Task) {
        let source_path: PathBuf = Path::new(".").join("SavedFile").join(&task.image_filename);
        let destination_path: PathBuf = Path::new(".").join("PreProcessing").join(&task.image_filename);
        let create_folder: PathBuf = destination_path.clone().with_extension("").to_path_buf();
        if let Err(err) = fs::create_dir(&create_folder).await {
            Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Failed to create directory {}: {:?}", create_folder.display(), err));
            return;
        }
        if let Err(err) = fs::rename(&source_path, &destination_path).await {
            Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Failed to move file from {} to {}: {:?}", source_path.display(), destination_path.display(), err));
            return;
        }
        let zip_path = destination_path.clone();
        let result = tokio::task::spawn_blocking(move || {
            Self::zip_process(zip_path)
        }).await;
        match result {
            Ok(Ok(_)) => {
                match Self::file_count(create_folder).await {
                    Ok(count) => {
                        task.unprocessed = count;
                        Self::task_manager_process(task).await;
                    }
                    Err(err) => Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Error reading directory: {}.", err))
                }
            },
            Ok(Err(err)) => Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Zip extraction failed: {}.", err)),
            Err(err) => Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Task panicked: {:?}.", err)),
        }
    }

    fn zip_process(zip_path: PathBuf) -> Result<(), String> {
        let allowed_extensions = ["png", "jpg", "jpeg"];
        let reader = match File::open(&zip_path) {
            Ok(r) => r,
            Err(err) => return Err(format!("Failed to open ZIP file: {}", err)),
        };
        let mut archive = match ZipArchive::new(reader) {
            Ok(archive) => archive,
            Err(err) => return Err(format!("Failed to read ZIP archive: {}", err)),
        };
        let output_folder = zip_path.clone().with_extension("").to_path_buf();
        for i in 0..archive.len() {
            let mut file = match archive.by_index(i) {
                Ok(file) => file,
                Err(err) => return Err(format!("Failed to access ZIP entry by index: {}", err)),
            };
            if let Some(enclosed_path) = file.enclosed_name() {
                if let Some(extension) = enclosed_path.extension() {
                    if allowed_extensions.contains(&extension.to_str().unwrap_or("")) {
                        let output_filepath = output_folder.join(enclosed_path.file_name().unwrap_or_default());
                        let mut output_file = match File::create(&output_filepath) {
                            Ok(file) => file,
                            Err(err) => return Err(format!("Failed to create output file: {}", err)),
                        };
                        if let Err(err) = std::io::copy(&mut file, &mut output_file) {
                            return Err(format!("Failed to write to output file: {}", err));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn file_count(path: PathBuf) -> Result<usize, Error> {
        let mut dir_entries = fs::read_dir(path).await?;
        let mut count = 0;
        while let Some(entry) = dir_entries.next_entry().await? {
            if entry.path().is_file() {
                count += 1;
            }
        }
        Ok(count)
    }

    async fn task_manager_process(mut task: Task) {
        task.status = TaskStatus::Waiting;
        TaskManager::add_task(task).await;
    }
}
