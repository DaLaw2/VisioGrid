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
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use rusttype::{Font, Scale};
use tokio::task::{JoinError, spawn_blocking};
use crate::manager::result_repository::ResultRepository;
use crate::utils::config::Config;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::task_manager::TaskManager;
use crate::manager::utils::image_task::ImageTask;
use crate::manager::utils::task::{Task, TaskStatus};

lazy_static! {
    static ref GLOBAL_FILE_MANAGER: RwLock<FileManager> = RwLock::new(FileManager::new());
}

pub struct FileManager {
    pre_processing: VecDeque<Task>,
    post_processing: VecDeque<Task>
}

impl FileManager {
    fn new() -> Self {
        Self {
            pre_processing: VecDeque::new(),
            post_processing: VecDeque::new()
        }
    }

    pub async fn initialize() {
        let folders = ["SavedModel", "SavedFile", "PreProcessing", "PostProcessing", "Result"];
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
        let folders = ["SavedModel", "SavedFile", "PreProcessing", "PostProcessing", "Result"];
        for &folder_name in &folders {
            match fs::remove_dir_all(folder_name).await {
                Ok(_) => Logger::append_system_log(LogLevel::INFO, format!("File Manager: Deleted {} folder successfully.", folder_name)).await,
                Err(_) => Logger::append_system_log(LogLevel::ERROR, format!("File Manager: Cannot delete {} folder.", folder_name)).await
            }
        };
    }

    pub async fn run() {
        tokio::spawn(async {
            Self::pre_processing().await
        });
        tokio::spawn(async {
            Self::post_processing().await
        });
    }

    pub async fn add_pre_process_task(task: Task) {
        let mut manager = GLOBAL_FILE_MANAGER.write().await;
        manager.pre_processing.push_front(task);
    }

    pub async fn add_post_process_task(task: Task) {
        let mut manager = GLOBAL_FILE_MANAGER.write().await;
        manager.post_processing.push_front(task);
    }

    async fn pre_processing() {
        let config = Config::now().await;
        loop {
            let task = GLOBAL_FILE_MANAGER.write().await.pre_processing.pop_back();
            match task {
                Some(mut task) => {
                    task.change_status(TaskStatus::PreProcessing);
                    match Path::new(&task.media_filename).extension().and_then(OsStr::to_str) {
                        Some("png") | Some("jpg") | Some("jpeg") => Self::picture_pre_processing(task).await,
                        Some("gif") | Some("mp4") | Some("wav") | Some("avi") | Some("mkv") => Self::media_pre_process(task).await,
                        Some("zip") => Self::zip_pre_process(task).await,
                        _ => {
                            let error_message = format!("File Manager: Task {} failed because the file extension is not supported.", task.uuid);
                            task.update_unprocessed(Err(error_message.clone())).await;
                            Logger::append_system_log(LogLevel::ERROR, error_message).await;
                        },
                    }
                },
                None => sleep(Duration::from_millis(config.internal_timestamp)).await
            }
        }
    }

    async fn post_processing() {
        let config = Config::now().await;
        loop {
            let task = GLOBAL_FILE_MANAGER.write().await.post_processing.pop_back();
            match task {
                Some(mut task) => {
                    task.change_status(TaskStatus::PostProcessing);
                    match Path::new(&task.media_filename).extension().and_then(OsStr::to_str) {
                        Some("png") | Some("jpg") | Some("jpeg") => Self::picture_post_processing(task).await,
                        Some("gif") | Some("mp4") | Some("wav") | Some("avi") | Some("mkv") => Self::recombination_media(task).await,
                        Some("zip") => Self::recombination_zip(task).await,
                        _ => {
                            let error_message = format!("File Manager: Task {} failed because the file extension is not supported.", task.uuid);
                            task.update_unprocessed(Err(error_message.clone())).await;
                            Logger::append_system_log(LogLevel::ERROR, error_message).await;
                        },
                    }
                },
                None => sleep(Duration::from_millis(config.internal_timestamp)).await
            }
        }
    }

    async fn picture_pre_processing(mut task: Task) {
        let source_path = Path::new(".").join("SavedFile").join(&task.media_filename);
        let destination_path = Path::new(".").join("PreProcessing").join(&task.media_filename);
        match fs::rename(source_path, destination_path).await {
            Ok(_) => {
                task.update_unprocessed(Ok(1)).await;
                Self::forward_to_task_manager(task).await;
            },
            Err(_) => {
                let error_message = format!("File Manager: Task {} failed because move image file failed.", task.uuid);
                task.update_unprocessed(Err(error_message.clone())).await;
                Logger::append_system_log(LogLevel::INFO, error_message).await;
            }
        }
    }

    async fn extract_pre_processing(task: &mut Task, source_path: PathBuf, destination_path: &PathBuf, create_folder: &PathBuf)  {
        if let Err(_) = fs::create_dir(&create_folder).await {
            let error_message = format!("File Manager: Cannot create {} folder.", create_folder.display());
            task.update_unprocessed(Err(error_message.clone())).await;
            Logger::append_system_log(LogLevel::INFO, error_message).await;
            return;
        }
        if let Err(_) = fs::rename(&source_path, &destination_path).await {
            let error_message = format!("File Manager: Cannot to move file from {} to {}.", source_path.display(), destination_path.display());
            task.update_unprocessed(Err(error_message.clone())).await;
            Logger::append_system_log(LogLevel::INFO, error_message).await;
            return;
        }
    }

    async fn extract_post_processing(mut task: Task, created_folder: PathBuf, result: Result<Result<(), String>, JoinError>) {
        match result {
            Ok(Ok(_)) => {
                match Self::file_count(&created_folder).await {
                    Ok(count) => {
                        task.update_unprocessed(Ok(count)).await;
                        Self::forward_to_task_manager(task).await;
                    }
                    Err(_) => {
                        let error_message = format!("File Manager: An error occurred while reading folder {}.", created_folder.display());
                        task.update_unprocessed(Err(error_message.clone())).await;
                        Logger::append_system_log(LogLevel::INFO, error_message).await;
                    }
                }
            },
            Ok(Err(err)) => {
                task.update_unprocessed(Err(err.clone())).await;
                Logger::append_system_log(LogLevel::INFO, err).await;
            },
            Err(_) => {
                let error_message = format!("File Manager: Task {} panic.", task.uuid);
                task.update_unprocessed(Err(error_message.clone())).await;
                Logger::append_system_log(LogLevel::INFO, error_message).await;
            },
        }
    }

    async fn media_pre_process(mut task: Task) {
        let source_path = Path::new(".").join("SavedFile").join(&task.media_filename);
        let destination_path = Path::new(".").join("PreProcessing").join(&task.media_filename);
        let create_folder = destination_path.clone().with_extension("");
        Self::extract_pre_processing(&mut task, source_path, &destination_path, &create_folder).await;
        let media_path = destination_path;
        let result = spawn_blocking(move || {
            Self::extract_media(media_path)
        }).await;
        Self::extract_post_processing(task, create_folder, result).await;
    }

    fn extract_media(media_path: PathBuf) -> Result<(), String> {
        let saved_path = media_path.clone().with_extension("").to_path_buf();
        let pipeline_string = format!("filesrc location={:?} ! decodebin ! videoconvert ! pngenc ! multifilesink location={:?}", media_path, saved_path.join("%d.jpeg"));
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

    async fn zip_pre_process(mut task: Task) {
        let source_path = Path::new(".").join("SavedFile").join(&task.media_filename);
        let destination_path = Path::new(".").join("PreProcessing").join(&task.media_filename);
        let create_folder = destination_path.clone().with_extension("");
        Self::extract_pre_processing(&mut task, source_path, &destination_path, &create_folder).await;
        let zip_path = destination_path;
        let result = spawn_blocking(move || {
            Self::extract_zip(&zip_path)
        }).await;
        Self::extract_post_processing(task, create_folder, result).await;
    }

    fn extract_zip(zip_path: &PathBuf) -> Result<(), String> {
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

    async fn draw_bounding_box(image_task: &mut ImageTask) -> Result<RgbImage, String> {
        let config = Config::now().await;
        let border_color = Rgb(config.border_color);
        let text_color = Rgb(config.text_color);
        let mut font_data = fs::read(config.font_path).await;
        let image_path = image_task.image_filepath.clone();
        match image::open(image_path) {
            Ok(image) => {
                let mut image = image.to_rgb8();
                for bounding_box in &mut image_task.bounding_boxes {
                    let base_rectangle = Rect::at(bounding_box.x1 as i32, bounding_box.y1 as i32).of_size(bounding_box.x2 - bounding_box.x1, bounding_box.y2 - bounding_box.y1);
                    for i in 0..config.border_width {
                        let offset_rect = Rect::at(base_rectangle.left() - i as i32, base_rectangle.top() - i as i32).of_size(base_rectangle.width() + 2 * i, base_rectangle.height() + 2 * i);
                        draw_hollow_rect_mut(&mut image, offset_rect, border_color);
                    }
                    if let Ok(font_data) = &mut font_data {
                        if let Some(font) = Font::try_from_bytes(font_data) {
                            let scale = Scale::uniform(config.font_size);
                            let text = format!("{}: {:2}%", bounding_box.name, bounding_box.confidence);
                            let position_x = bounding_box.x1 as i32;
                            let position_y = (bounding_box.y2 + config.border_width + 10) as i32;
                            draw_text_mut(&mut image, text_color, position_x, position_y, scale, &font, &text);
                        }
                    }
                }
                Ok(image)
            },
            Err(_) => Err(format!("FileManager: Cannot read file {:?}.", image_path)),
        }
    }

    async fn picture_post_processing(mut task: Task) {
        match task.result.get(0) {
            Some(task) => {
                let source_path = task.image_filepath.clone();
                let destination_path = Path::new(".").join("PostProcessing").join(task.image_filename.clone())
            },
            None => {
                Logger::append_system_log(LogLevel::ERROR, "FileManager: Image Task disappeared.".to_string()).await;
                ResultRepository::add_task(task).await;
            }
        }
    }

    async fn recombination_pre_processing() {

    }

    async fn media_post_processing(mut task: Task) {

    }

    async fn recombination_media() {

    }

    async fn zip_post_processing(mut task: Task) {

    }

    async fn recombination_zip() {

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

    async fn forward_to_task_manager(mut task: Task) {
        task.change_status(TaskStatus::Waiting);
        TaskManager::add_task(task).await;
    }
}
