use std::fs::File;
use std::io::Error;
use std::ffi::OsStr;
use tokio::{fs, io};
use tokio::time::sleep;
use std::time::Duration;
use gstreamer::prelude::*;
use zip::read::ZipArchive;
use imageproc::rect::Rect;
use image::{Rgb, RgbImage};
use rusttype::{Font, Scale};
use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use gstreamer_pbutils::prelude::*;
use gstreamer_pbutils::Discoverer;
use tokio::task::{JoinError, spawn_blocking};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use imageproc::drawing::{draw_hollow_rect_mut, draw_text_mut};
use crate::utils::config::Config;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::task_manager::TaskManager;
use crate::manager::utils::video_info::VideoInfo;
use crate::manager::utils::image_task::ImageTask;
use crate::manager::utils::task::{Task, TaskStatus};
use crate::manager::result_repository::ResultRepository;

lazy_static! {
    static ref GLOBAL_FILE_MANAGER: RwLock<FileManager> = RwLock::new(FileManager::new());
}

pub struct FileManager {
    pre_processing: VecDeque<Task>,
    post_processing: VecDeque<Task>,
    terminate: bool,
}

impl FileManager {
    fn new() -> Self {
        Self {
            pre_processing: VecDeque::new(),
            post_processing: VecDeque::new(),
            terminate: false,
        }
    }

    pub async fn instance() -> RwLockReadGuard<'static, Self> {
        GLOBAL_FILE_MANAGER.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        GLOBAL_FILE_MANAGER.write().await
    }

    pub async fn run() {
        Self::initialize().await;
        tokio::spawn(async {
            Self::pre_processing().await;
        });
        tokio::spawn(async {
            Self::post_processing().await;
        });
    }

    pub async fn terminate() {
        Self::instance_mut().await.terminate = true;
        Self::cleanup().await;
    }

    async fn initialize() {
        let folders = ["SavedModel", "SavedFile", "PreProcessing", "PostProcessing", "Result"];
        for &folder_name in &folders {
            match fs::create_dir(folder_name).await {
                Ok(_) => Logger::append_system_log(LogLevel::INFO, format!("File Manager: Create {} folder successfully.", folder_name)).await,
                Err(err) => Logger::append_system_log(LogLevel::ERROR, format!("File Manager: Cannot create {} folder.\nReason: {}.", folder_name, err)).await
            }
        }
        match gstreamer::init() {
            Ok(_) => Logger::append_system_log(LogLevel::INFO, "File Manager: GStreamer initialization successfully.".to_string()).await,
            Err(err) => Logger::append_system_log(LogLevel::ERROR, format!("File Manager: GStreamer initialization failed.\nReason: {}.", err)).await,
        }
    }

    async fn cleanup() {
        let folders = ["SavedModel", "SavedFile", "PreProcessing", "PostProcessing", "Result"];
        for &folder_name in &folders {
            match fs::remove_dir_all(folder_name).await {
                Ok(_) => Logger::append_system_log(LogLevel::INFO, format!("File Manager: Deleted {} folder successfully.", folder_name)).await,
                Err(err) => Logger::append_system_log(LogLevel::ERROR, format!("File Manager: Cannot delete {} folder.\nReason: {}.", folder_name, err)).await
            }
        };
    }

    pub async fn add_pre_process_task(task: Task) {
        Self::instance_mut().await.pre_processing.push_front(task);
    }

    pub async fn add_post_process_task(task: Task) {
        Self::instance_mut().await.post_processing.push_front(task);
    }

    async fn pre_processing() {
        let config = Config::now().await;
        loop {
            if Self::instance().await.terminate {
                return;
            }
            let task = Self::instance_mut().await.pre_processing.pop_back();
            match task {
                Some(mut task) => {
                    task.change_status(TaskStatus::PreProcessing);
                    match Path::new(&task.media_filename).extension().and_then(OsStr::to_str) {
                        Some("png") | Some("jpg") | Some("jpeg") => Self::picture_pre_processing(task).await,
                        Some("mp4") | Some("wav") | Some("avi") | Some("mkv") => Self::video_pre_process(task).await,
                        Some("zip") => Self::zip_pre_process(task).await,
                        _ => {
                            let error_message = format!("File Manager: Task {} failed because the file extension is not supported.", task.uuid);
                            task.update_unprocessed(Err(error_message.clone())).await;
                            Logger::append_system_log(LogLevel::ERROR, error_message).await;
                        }
                    }
                }
                None => sleep(Duration::from_millis(config.internal_timestamp)).await
            }
        }
    }

    async fn post_processing() {
        let config = Config::now().await;
        loop {
            if Self::instance().await.terminate {
                return;
            }
            let task = Self::instance_mut().await.post_processing.pop_back();
            match task {
                Some(mut task) => {
                    task.change_status(TaskStatus::PostProcessing);
                    match Path::new(&task.media_filename).extension().and_then(OsStr::to_str) {
                        Some("png") | Some("jpg") | Some("jpeg") => Self::picture_post_processing(task).await,
                        Some("mp4") | Some("wav") | Some("avi") | Some("mkv") => Self::video_post_processing(task).await,
                        Some("zip") => Self::zip_post_processing(task).await,
                        _ => {
                            let error_message = format!("File Manager: Task {} failed because the file extension is not supported.", task.uuid);
                            task.update_unprocessed(Err(error_message.clone())).await;
                            Logger::append_system_log(LogLevel::ERROR, error_message).await;
                        }
                    }
                }
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
            }
            Err(err) => {
                let error_message = format!("File Manager: Task {} failed because move image file failed.\nReason: {}.", task.uuid, err);
                task.update_unprocessed(Err(error_message.clone())).await;
                Logger::append_system_log(LogLevel::INFO, error_message).await;
            }
        }
    }

    async fn extract_pre_processing(task: &mut Task, source_path: &PathBuf, destination_path: &PathBuf, create_folder: &PathBuf) {
        if let Err(err) = fs::create_dir(&create_folder).await {
            let error_message = format!("File Manager: Cannot create {} folder.\nReason: {}.", create_folder.display(), err);
            task.update_unprocessed(Err(error_message.clone())).await;
            Logger::append_system_log(LogLevel::INFO, error_message).await;
            return;
        }
        if let Err(err) = fs::rename(&source_path, &destination_path).await {
            let error_message = format!("File Manager: Cannot to move file from {} to {}.\nReason: {}.", source_path.display(), destination_path.display(), err);
            task.update_unprocessed(Err(error_message.clone())).await;
            Logger::append_system_log(LogLevel::INFO, error_message).await;
            return;
        }
    }

    async fn extract_process_result(mut task: Task, created_folder: PathBuf, result: Result<Result<(), String>, JoinError>) {
        match result {
            Ok(Ok(_)) => {
                match Self::file_count(&created_folder).await {
                    Ok(count) => {
                        task.update_unprocessed(Ok(count)).await;
                        Self::forward_to_task_manager(task).await;
                    }
                    Err(err) => {
                        let error_message = format!("File Manager: An error occurred while reading folder {}.\nReason: {}.", created_folder.display(), err);
                        task.update_unprocessed(Err(error_message.clone())).await;
                        Logger::append_system_log(LogLevel::INFO, error_message).await;
                    }
                }
            }
            Ok(Err(err)) => {
                task.update_unprocessed(Err(err.clone())).await;
                Logger::append_system_log(LogLevel::INFO, err).await;
            }
            Err(err) => {
                let error_message = format!("File Manager: Task {} panic.\nReason: {}.", task.uuid, err);
                task.update_unprocessed(Err(error_message.clone())).await;
                Logger::append_system_log(LogLevel::ERROR, error_message).await;
            }
        }
    }

    async fn video_pre_process(mut task: Task) {
        let source_path = Path::new(".").join("SavedFile").join(&task.media_filename);
        let destination_path = Path::new(".").join("PreProcessing").join(&task.media_filename);
        let create_folder = destination_path.clone().with_extension("");
        Self::extract_pre_processing(&mut task, &source_path, &destination_path, &create_folder).await;
        let video_path = destination_path;
        if let Err(err) = Self::extract_video_info(video_path.clone()).await {
            Logger::append_system_log(LogLevel::ERROR, err).await;
        }
        let result = spawn_blocking(move || {
            Self::extract_video(video_path)
        }).await;
        Self::extract_process_result(task, create_folder, result).await;
    }

    async fn extract_video_info(video_path: PathBuf) -> Result<(), String> {
        let absolute_path = video_path.canonicalize()
            .map_err(|err| format!("File Manager: Unable to get absolute path.\nReason: {}.", err))?;
        let clean_path = absolute_path.to_string_lossy().trim_start_matches(r"\\?\").replace("\\", "/");
        let discoverer = Discoverer::new(gstreamer::ClockTime::from_seconds(5))
            .map_err(|err| format!("File Manager: Failed to create Discoverer element.\nReason: {}.", err))?;
        let info = discoverer.discover_uri(&*format!("file:///{}", clean_path))
            .map_err(|err| format!("File Manager: Failed to get video info.\nReason: {}.", err))?;
        let mut video_info = VideoInfo::default();
        if let Some(stream) = info.video_streams().get(0) {
            if let Some(caps) = stream.caps() {
                if let Some(structure) = caps.structure(0) {
                    video_info.format = structure.name().to_string();
                    for field in structure.fields() {
                        match field.as_str() {
                            "stream-format" => video_info.stream_format = structure.get::<String>(field).unwrap_or_default(),
                            "alignment" => video_info.alignment = structure.get::<String>(field).unwrap_or_default(),
                            "level" => video_info.level = structure.get::<String>(field).unwrap_or_default(),
                            "profile" => video_info.profile = structure.get::<String>(field).unwrap_or_default(),
                            "width" => video_info.width = structure.get::<i32>(field).unwrap_or_default(),
                            "height" => video_info.height = structure.get::<i32>(field).unwrap_or_default(),
                            "framerate" => video_info.framerate = structure.get::<gstreamer::Fraction>(field).map_or_else(|_| "0/1".to_string(), |f| f.to_string()),
                            "pixel-aspect-ratio" => video_info.pixel_aspect_ratio = structure.get::<gstreamer::Fraction>(field).map_or_else(|_| "1/1".to_string(), |f| f.to_string()),
                            "coded-picture-structure" => video_info.coded_picture_structure = structure.get::<String>(field).unwrap_or_default(),
                            "chroma-format" => video_info.chroma_format = structure.get::<String>(field).unwrap_or_default(),
                            "bit-depth-luma" => video_info.bit_depth_luma = structure.get::<u32>(field).unwrap_or_default(),
                            "bit-depth-chroma" => video_info.bit_depth_chroma = structure.get::<u32>(field).unwrap_or_default(),
                            "colorimetry" => video_info.colorimetry = structure.get::<String>(field).unwrap_or_default(),
                            _ => (),
                        }
                    }
                }
            }
        }
        let toml_path = video_path.with_extension("toml");
        let toml_string = toml::to_string(&video_info)
            .map_err(|err| format!("File Manager: Unable to serialize video info.\nReason: {}.", err))?;
        fs::write(&toml_path, toml_string).await
            .map_err(|err| format!("File Manager: Unable to write video info to TOML file.\nReason: {}.", err))?;
        Ok(())
    }

    fn extract_video(video_path: PathBuf) -> Result<(), String> {
        let saved_path = video_path.clone().with_extension("").to_path_buf();
        let pipeline_string = format!("filesrc location={:?} ! decodebin ! videoconvert ! pngenc ! multifilesink location={:?}", video_path, saved_path.join("%d.png"));
        let pipeline = gstreamer::parse_launch(&pipeline_string)
            .map_err(|err| format!("File Manager: GStreamer cannot parse pipeline.\nReason: {}.", err))?;
        let bus = pipeline.bus().ok_or("File Manager: Unable to get pipeline bus.".to_string())?;
        pipeline.set_state(gstreamer::State::Playing)
            .map_err(|err| format!("File Manager: Unable to set pipeline to playing.\nReason: {}.", err))?;
        for message in bus.iter_timed(gstreamer::ClockTime::NONE) {
            match message.view() {
                gstreamer::MessageView::Eos(..) => break,
                gstreamer::MessageView::Error(_) => {
                    return if let Some(src) = message.src() {
                        let path = src.path_string();
                        Err(format!("File Manager: An error occurred in gstreamer.\nError from {}.", path))
                    } else {
                        Err("File Manager: An unknown error occurred in gstreamer.".to_string())
                    };
                }
                _ => (),
            }
        }
        pipeline.set_state(gstreamer::State::Null)
            .map_err(|err| format!("File Manager: Unable to set pipeline to null.\nReason: {}.", err))?;
        Ok(())
    }

    async fn zip_pre_process(mut task: Task) {
        let source_path = Path::new(".").join("SavedFile").join(&task.media_filename);
        let destination_path = Path::new(".").join("PreProcessing").join(&task.media_filename);
        let create_folder = destination_path.clone().with_extension("");
        Self::extract_pre_processing(&mut task, &source_path, &destination_path, &create_folder).await;
        let zip_path = destination_path;
        let result = spawn_blocking(move || {
            Self::extract_zip(&zip_path)
        }).await;
        Self::extract_process_result(task, create_folder, result).await;
    }

    fn extract_zip(zip_path: &PathBuf) -> Result<(), String> {
        let allowed_extensions = ["png", "jpg", "jpeg"];
        let reader = File::open(&zip_path)
            .map_err(|_| format!("File Manager: Unable to open ZIP file {}.", zip_path.display()))?;
        let mut archive = ZipArchive::new(reader)
            .map_err(|err| format!("File Manager: Unable to read {} archive.\nReason: {}.", zip_path.display(), err))?;
        let output_folder = zip_path.clone().with_extension("").to_path_buf();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|err| format!("File Manager: Unable to access {} entry by index.\nReason: {}.", zip_path.display(), err))?;
            if let Some(enclosed_path) = file.enclosed_name() {
                if let Some(extension) = enclosed_path.extension() {
                    if allowed_extensions.contains(&extension.to_str().unwrap_or("")) {
                        let output_filepath = output_folder.join(enclosed_path.file_name().unwrap_or_default());
                        let mut output_file = File::create(&output_filepath)
                            .map_err(|_| format!("File Manager: Cannot create output file {}: ", output_filepath.display()))?;
                        std::io::copy(&mut file, &mut output_file)
                            .map_err(|err| format!("File Manager: Unable to write to output file {}.\nReason: {}.", output_filepath.display(), err))?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn draw_bounding_box(image_task: &ImageTask, config: &Config, font_data: &io::Result<Vec<u8>>) -> Result<RgbImage, String> {
        let border_color = Rgb(config.border_color);
        let text_color = Rgb(config.text_color);
        let image_path = image_task.image_filepath.clone();
        let mut image = image::open(image_path)
            .map_err(|err| format!("FileManager: Cannot read file {}.\nReason: {}.", image_task.image_filepath.display(), err))?
            .to_rgb8();
        for bounding_box in &image_task.bounding_boxes {
            let base_rectangle = Rect::at(bounding_box.x1 as i32, bounding_box.y1 as i32).of_size(bounding_box.x2 - bounding_box.x1, bounding_box.y2 - bounding_box.y1);
            for i in 0..config.border_width {
                let offset_rect = Rect::at(base_rectangle.left() - i as i32, base_rectangle.top() - i as i32).of_size(base_rectangle.width() + 2 * i, base_rectangle.height() + 2 * i);
                draw_hollow_rect_mut(&mut image, offset_rect, border_color);
            }
            if let Ok(font_data) = &font_data {
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
    }

    async fn picture_post_processing(task: Task) {
        match task.result.get(0) {
            Some(image_task) => {
                let config = Config::now().await;
                let font_data = fs::read(&config.font_path).await;
                let saved_path = Path::new(".").join("PostProcessing").join(image_task.image_filename.clone());
                match Self::draw_bounding_box(image_task, &config, &font_data).await {
                    Ok(image) => {
                        if let Err(err) = image.save(&saved_path) {
                            Logger::append_system_log(LogLevel::ERROR, format!("File Manager: Unable to write to output file {}.\nReason: {}.", saved_path.display(), err)).await;
                        }
                    }
                    Err(err) => Logger::append_system_log(LogLevel::ERROR, err).await,
                }
            },
            None => {
                //Impossible, it means that an ImageTask is missing.
                Logger::append_system_log(LogLevel::ERROR, "FileManager: Internal server error.\nReason: Image Task Disappeared.".to_string()).await;
                ResultRepository::task_failed(task).await;
            }
        }
    }

    async fn recombination_pre_processing(task: &mut Task, create_folder: PathBuf) {
        if let Err(err) = fs::create_dir(&create_folder).await {
            let error_message = format!("File Manager: Cannot create {} folder.\nReason: {}.", create_folder.display(), err);
            task.panic(error_message.clone()).await;
            Logger::append_system_log(LogLevel::INFO, error_message).await;
            return;
        }
        for image_task in &task.result {
            let config = Config::now().await;
            let font_data = fs::read(&config.font_path).await;
            let saved_path = create_folder.join(image_task.image_filename.clone());
            match Self::draw_bounding_box(image_task, &config, &font_data).await {
                Ok(image) => {
                    if let Err(err) = image.save(&saved_path) {
                        Logger::append_system_log(LogLevel::ERROR, format!("File Manager: Unable to write to output file {}.\nReason: {}.", saved_path.display(), err)).await;
                    }
                }
                Err(err) => Logger::append_system_log(LogLevel::ERROR, err).await,
            }
        }
    }

    async fn video_post_processing(mut task: Task) {

    }

    async fn recombination_video() {

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
