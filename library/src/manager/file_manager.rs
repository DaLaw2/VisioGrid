use tokio::fs;
use std::fs::File;
use std::io::Error;
use std::ffi::OsStr;
use tokio::time::sleep;
use tokio::sync::RwLock;
use std::time::Duration;
use gstreamer::prelude::*;
use zip::read::ZipArchive;
use lazy_static::lazy_static;
use std::path::{Path, PathBuf};
use std::collections::VecDeque;
use crate::utils::config::Config;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::task_manager::TaskManager;
use crate::manager::utils::task::{Task, TaskStatus};
use crate::manager::utils::task::TaskStatus::Fail;
use crate::manager::result_repository::ResultRepository;

lazy_static! {
    static ref GLOBAL_FILE_MANAGER: RwLock<FileManager> = RwLock::new(FileManager::new());
}

pub struct FileManager {
    preprocessing: VecDeque<Task>,
    postprocessing: VecDeque<Task>
}

impl FileManager {
    fn new() -> Self {
        Self {
            preprocessing: VecDeque::new(),
            postprocessing: VecDeque::new()
        }
    }

    pub async fn initialize() {
        let folders = ["SavedModel", "SavedFile", "PreProcessing", "Processing", "PostProcessing", "Result"];
        for &folder_name in &folders {
            match fs::create_dir(folder_name).await {
                Ok(_) => Logger::append_system_log(LogLevel::INFO, format!("File Manager: Create {} folder successfully.", folder_name)).await,
                Err(_) => Logger::append_system_log(LogLevel::ERROR, format!("File Manager: Cannot create {} folder.", folder_name)).await
            }
        }
        match gstreamer::init() {
            Ok(_) => Logger::append_system_log(LogLevel::INFO, "File Manager: GStreamer initialization successfully.".to_string()).await,
            Err(err) => Logger::append_system_log(LogLevel::ERROR, format!("File Manager: GStreamer initialization failed: {:?}.", err)).await,
        }
    }

    pub async fn cleanup() {
        let folders = ["SavedModel", "SavedFile", "PreProcessing", "Processing", "PostProcessing", "Result"];
        for &folder_name in &folders {
            match fs::remove_dir_all(folder_name).await {
                Ok(_) => Logger::append_system_log(LogLevel::INFO, format!("File Manager: Successfully deleted {} folder.", folder_name)).await,
                Err(_) => Logger::append_system_log(LogLevel::ERROR, format!("File Manager: Failed to delete {} folder.", folder_name)).await
            }
        };
    }

    pub async fn run() {
        tokio::spawn(async {
            Self::preprocessing().await
        });
        tokio::spawn(async {
            Self::postprocessing().await
        });
    }

    pub async fn add_preprocess_task(task: Task) {
        let mut manager = GLOBAL_FILE_MANAGER.write().await;
        manager.preprocessing.push_back(task);
    }

    pub async fn add_postprocess_task(task: Task) {
        let mut manager = GLOBAL_FILE_MANAGER.write().await;
        manager.postprocessing.push_back(task);
    }

    async fn preprocessing() {
        let config = Config::now().await;
        loop {
            let task = GLOBAL_FILE_MANAGER.write().await.preprocessing.pop_front();
            match task {
                Some(mut task) => {
                    match Path::new(&task.image_filename).extension().and_then(OsStr::to_str) {
                        Some("png") | Some("jpg") | Some("jpeg") => {
                            let source_path = Path::new(".").join("SavedFile").join(&task.image_filename);
                            let destination_path = Path::new(".").join("PreProcessing").join(&task.image_filename);
                            match fs::rename(source_path, destination_path).await {
                                Ok(_) => {
                                    FileManager::update_task_unprocessed(&mut task, Ok(1)).await;
                                    Self::task_manager_process(task).await
                                },
                                Err(_) => {
                                    let error_message = format!("File Manager: Task {} failed because move image file failed.", task.uuid);
                                    FileManager::update_task_unprocessed(&mut task, Err(error_message.clone())).await;
                                    Logger::append_system_log(LogLevel::ERROR, error_message).await;
                                }
                            }
                        },
                        Some("gif") | Some("mp4") | Some("wav") | Some("avi") | Some("mkv") => Self::extract_media(task).await,
                        Some("zip") => Self::extract_zip(task).await,
                        _ => {
                            let error_message = format!("File Manager: Task {} failed because the file extension is not supported.", task.uuid);
                            FileManager::update_task_unprocessed(&mut task, Err(error_message.clone())).await;
                            Logger::append_system_log(LogLevel::INFO, error_message).await;
                        },
                    }
                },
                None => sleep(Duration::from_millis(config.internal_timestamp as u64)).await
            }
        }
    }

    async fn postprocessing() {

    }

    async fn extract_media(mut task: Task) {
        let source_path: PathBuf = Path::new(".").join("SavedFile").join(&task.image_filename);
        let destination_path: PathBuf = Path::new(".").join("PreProcessing").join(&task.image_filename);
        let create_folder: PathBuf = destination_path.clone().with_extension("");
        if let Err(_) = fs::create_dir(&create_folder).await {
            let error_message = format!("File Manager: Cannot create {} folder.", create_folder.display());
            FileManager::update_task_unprocessed(&mut task, Err(error_message.clone())).await;
            Logger::append_system_log(LogLevel::ERROR, error_message).await;
            return;
        }
        if let Err(_) = fs::rename(&source_path, &destination_path).await {
            let error_message = format!("File Manager: Cannot to move file from {} to {}", source_path.display(), destination_path.display());
            FileManager::update_task_unprocessed(&mut task, Err(error_message.clone())).await;
            Logger::append_system_log(LogLevel::ERROR, error_message).await;
            return;
        }
        let media_path = destination_path;
        let result = tokio::task::spawn_blocking(move || {
            Self::media_process(media_path)
        }).await;
        match result {
            Ok(Ok(_)) => {
                match Self::file_count(&create_folder).await {
                    Ok(count) => {
                        FileManager::update_task_unprocessed(&mut task, Ok(count)).await;
                        Self::task_manager_process(task).await;
                    }
                    Err(_) => {
                        let error_message = format!("File Manager: An error occurred while reading folder {}.", create_folder.display());
                        FileManager::update_task_unprocessed(&mut task, Err(error_message.clone())).await;
                        Logger::append_system_log(LogLevel::ERROR, error_message).await;
                    }
                }
            },
            Ok(Err(err)) => {
                FileManager::update_task_unprocessed(&mut task, Err(err.clone())).await;
                Logger::append_system_log(LogLevel::ERROR, err).await;
            },
            Err(_) => {
                let error_message = format!("File Manager: Task {} panic.", task.uuid);
                FileManager::update_task_unprocessed(&mut task, Err(error_message.clone())).await;
                Logger::append_system_log(LogLevel::ERROR, error_message).await;
            },
        }
    }

    fn media_process(media_path: PathBuf) -> Result<(), String> {
        let saved_path = media_path.clone().with_extension("").to_path_buf();
        let pipeline_string = format!("filesrc location={:?} ! decodebin ! videoconvert ! pngenc ! multifilesink location={:?}", media_path, saved_path.join("%d.png"));
        let pipeline = match gstreamer::parse_launch(&pipeline_string) {
            Ok(pipeline) => pipeline,
            Err(_) => return Err("File Manager: GStreamer cannot parse pipeline.".to_string())
        };
        let bus = match pipeline.bus() {
            Some(bus) => bus,
            None => return Err("File Manager: Unable to get pipeline bus.".to_string())
        };
        if let Err(_) = pipeline.set_state(gstreamer::State::Playing) {
            return Err("File Manager: Unable to set pipeline to playing.".to_string());
        }
        for message in bus.iter_timed(gstreamer::ClockTime::NONE) {
            use gstreamer::MessageView;

            match message.view() {
                MessageView::Eos(..) => break,
                MessageView::Error(_) => {
                    return if let Some(src) = message.src() {
                        let path = src.path_string();
                        Err(format!("File Manager: Error from {}.", path))
                    } else {
                        Err("File Manager: Error from an unknown source.".to_string())
                    }
                },
                _ => (),
            }
        }
        if let Err(_) = pipeline.set_state(gstreamer::State::Null) {
            return Err("File Manager: Unable to set pipeline to null".to_string());
        }
        Ok(())
    }

    async fn extract_zip(mut task: Task) {
        let source_path: PathBuf = Path::new(".").join("SavedFile").join(&task.image_filename);
        let destination_path: PathBuf = Path::new(".").join("PreProcessing").join(&task.image_filename);
        let create_folder: PathBuf = destination_path.clone().with_extension("").to_path_buf();
        if let Err(_) = fs::create_dir(&create_folder).await {
            let error_message = format!("File Manager: Cannot create {} folder.", create_folder.display());
            FileManager::update_task_unprocessed(&mut task, Err(error_message.clone())).await;
            Logger::append_system_log(LogLevel::ERROR, error_message).await;
            return;
        }
        if let Err(_) = fs::rename(&source_path, &destination_path).await {
            let error_message = format!("File Manager: Cannot to move file from {} to {}.", source_path.display(), destination_path.display());
            FileManager::update_task_unprocessed(&mut task, Err(error_message.clone())).await;
            Logger::append_system_log(LogLevel::ERROR, error_message).await;
            return;
        }
        let zip_path = destination_path;
        let result = tokio::task::spawn_blocking(move || {
            Self::zip_process(&zip_path)
        }).await;
        match result {
            Ok(Ok(_)) => {
                match Self::file_count(&create_folder).await {
                    Ok(count) => {
                        FileManager::update_task_unprocessed(&mut task, Ok(count)).await;
                        Self::task_manager_process(task).await;
                    }
                    Err(_) => {
                        let error_message = format!("File Manager: An error occurred while reading folder {}.", create_folder.display());
                        FileManager::update_task_unprocessed(&mut task, Err(error_message.clone())).await;
                        Logger::append_system_log(LogLevel::ERROR, error_message).await;
                    }
                }
            },
            Ok(Err(err)) => {
                FileManager::update_task_unprocessed(&mut task, Err(err.clone())).await;
                Logger::append_system_log(LogLevel::ERROR, err).await;
            },
            Err(_) => {
                let error_message = format!("File Manager: Task {} panic.", task.uuid);
                FileManager::update_task_unprocessed(&mut task, Err(error_message.clone())).await;
                Logger::append_system_log(LogLevel::ERROR, error_message).await;
            },
        }
    }

    fn zip_process(zip_path: &PathBuf) -> Result<(), String> {
        let allowed_extensions = ["png", "jpg", "jpeg"];
        let reader = match File::open(&zip_path) {
            Ok(reader) => reader,
            Err(_) => return Err(format!("File Manager: Unable to open ZIP file {}.", zip_path.display())),
        };
        let mut archive = match ZipArchive::new(reader) {
            Ok(archive) => archive,
            Err(_) => return Err(format!("File Manager: Unable to read {} archive.", zip_path.display())),
        };
        let output_folder = zip_path.clone().with_extension("").to_path_buf();
        for i in 0..archive.len() {
            let mut file = match archive.by_index(i) {
                Ok(file) => file,
                Err(_) => return Err(format!("File Manager: Unable to access {} entry by index.", zip_path.display())),
            };
            if let Some(enclosed_path) = file.enclosed_name() {
                if let Some(extension) = enclosed_path.extension() {
                    if allowed_extensions.contains(&extension.to_str().unwrap_or("")) {
                        let output_filepath = output_folder.join(enclosed_path.file_name().unwrap_or_default());
                        let mut output_file = match File::create(&output_filepath) {
                            Ok(file) => file,
                            Err(_) => return Err(format!("File Manager: Cannot create output file {}: ", output_filepath.display())),
                        };
                        if let Err(err) = std::io::copy(&mut file, &mut output_file) {
                            return Err(format!("File Manager: Unable to write to output file {}.", err));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn update_task_unprocessed(task: &mut Task, result: Result<usize, String>) {
        match result {
            Ok(unprocessed) => task.unprocessed = unprocessed,
            Err(err) => {
                task.status = Fail;
                task.error = Err(err);
                ResultRepository::add_task(task.clone()).await;
            }
        }
    }

    async fn file_count(path: &PathBuf) -> Result<usize, Error> {
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
