use tokio::fs;
use std::fs::File;
use zip::ZipWriter;
use std::ffi::OsStr;
use futures::StreamExt;
use tokio::time::sleep;
use std::time::Duration;
use zip::read::ZipArchive;
use gstreamer::prelude::*;
use imageproc::rect::Rect;
use std::io::{Read, Write};
use image::{Rgb, RgbImage};
use zip::write::FileOptions;
use rusttype::{Font, Scale};
use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use gstreamer_pbutils::prelude::*;
use gstreamer_pbutils::Discoverer;
use tokio_stream::wrappers::ReadDirStream;
use tokio::task::{JoinError, spawn_blocking};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use imageproc::drawing::{draw_hollow_rect_mut, draw_text_mut};
use crate::utils::logging::*;
use crate::utils::config::Config;
use crate::management::task_manager::TaskManager;
use crate::management::utils::video_info::VideoInfo;
use crate::management::utils::image_task::ImageTask;
use crate::management::utils::task::{Task, TaskStatus};
use crate::management::result_repository::ResultRepository;

lazy_static! {
    static ref FILE_MANAGER: RwLock<FileManager> = RwLock::new(FileManager::new());
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
        FILE_MANAGER.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        FILE_MANAGER.write().await
    }

    pub async fn run() {
        Self::initialize().await;
        tokio::spawn(async {
            Self::pre_processing().await;
        });
        tokio::spawn(async {
            Self::post_processing().await;
        });
        logging_information!("File Manager", "Online now");
    }

    async fn initialize() {
        logging_information!("File Manager", "Initializing");
        let folders = ["SavedModel", "SavedFile", "PreProcessing", "PostProcessing", "Result"];
        for &folder_name in &folders {
            match fs::create_dir(folder_name).await {
                Ok(_) => logging_information!("File Manager", format!("Creating {folder_name} folder successfully")),
                Err(err) => logging_critical!("File Manager", format!("Cannot create folder {folder_name}"), format!("Err: {}", err)),
            }
        }
        match gstreamer::init() {
            Ok(_) => logging_information!("File Manager", "GStreamer initialization successful"),
            Err(err) => logging_critical!("File Manager", "GStreamer initialization failed", format!("Err: {}", err)),
        }
        logging_information!("File Manager", "Initialization completed");
    }

    pub async fn terminate() {
        logging_information!("File Manager", "Termination in progress");
        Self::instance_mut().await.terminate = true;
        Self::cleanup().await;
        logging_information!("File Manager", "Termination complete");
    }

    async fn cleanup() {
        logging_information!("File Manager", "Cleaning up");
        let folders = ["SavedModel", "SavedFile", "PreProcessing", "PostProcessing", "Result"];
        for &folder_name in &folders {
            match fs::remove_dir_all(folder_name).await {
                Ok(_) => logging_information!("File Manager", format!("Delete {folder_name} folder successfully")),
                Err(err) => logging_error!("File Manager", format!("Failed to delete {folder_name} folder"), format!("Err: {}", err)),
            }
        };
        logging_information!("File Manager", "Cleanup completed");
    }

    pub async fn add_pre_process_task(task: Task) {
        Self::instance_mut().await.pre_processing.push_back(task);
    }

    pub async fn add_post_process_task(task: Task) {
        Self::instance_mut().await.post_processing.push_back(task);
    }

    async fn pre_processing() {
        let config = Config::now().await;
        while !Self::instance().await.terminate {
            let task = Self::instance_mut().await.pre_processing.pop_front();
            match task {
                Some(mut task) => {
                    task.change_status(TaskStatus::PreProcessing);
                    match Path::new(&task.media_filename).extension().and_then(OsStr::to_str) {
                        Some("png") | Some("jpg") | Some("jpeg") => Self::picture_pre_processing(task).await,
                        Some("mp4") | Some("avi") | Some("mkv") => Self::video_pre_process(task).await,
                        Some("zip") => Self::zip_pre_process(task).await,
                        _ => {
                            logging_error!("File Manager", format!("Task {}, unsupported file type", task.uuid));
                            task.panic("Unsupported file type".to_string()).await;
                        }
                    }
                }
                None => sleep(Duration::from_millis(config.internal_timestamp)).await,
            }
        }
    }

    async fn post_processing() {
        let config = Config::now().await;
        while !Self::instance().await.terminate {
            let task = Self::instance_mut().await.post_processing.pop_front();
            match task {
                Some(mut task) => {
                    task.change_status(TaskStatus::PostProcessing);
                    match Path::new(&task.media_filename).extension().and_then(OsStr::to_str) {
                        Some("png") | Some("jpg") | Some("jpeg") => Self::picture_post_processing(task).await,
                        Some("mp4") | Some("avi") | Some("mkv") => Self::video_post_processing(task).await,
                        Some("zip") => Self::zip_post_processing(task).await,
                        _ => {
                            logging_error!("File Manager", format!("Task {}, unsupported file type", task.uuid));
                            task.panic("Unsupported file type".to_string()).await;
                        }
                    }
                }
                None => sleep(Duration::from_millis(config.internal_timestamp)).await,
            }
        }
    }

    async fn picture_pre_processing(mut task: Task) {
        let source_path = Path::new(".").join("SavedFile").join(&task.media_filename);
        let destination_path = Path::new(".").join("PreProcessing").join(&task.media_filename);
        match fs::rename(&source_path, &destination_path).await {
            Ok(_) => {
                task.update_unprocessed(1).await;
                Self::forward_to_task_manager(task).await;
            }
            Err(err) => {
                task.panic("Unable to move file".to_string()).await;
                logging_error!("File Manager", "Unable to move file".to_string(),
                    format!("Source: {}, Destination: {}, Err: {}", source_path.display(), destination_path.display(), err));
            }
        }
    }

    async fn extract_pre_processing(source_path: &PathBuf, destination_path: &PathBuf, create_folder: &PathBuf) -> Result<(), LogEntry> {
        fs::create_dir(&create_folder).await
            .map_err(|err| error_entry!("File Manager", &format!("Cannot create folder {}", create_folder.display()), format!("Err: {}", err)))?;
        fs::rename(&source_path, &destination_path).await
            .map_err(|err| {
                error_entry!("File Manager", "Unable to move file",
                    format!("Source: {}, Destination: {}, Err: {}", source_path.display(), destination_path.display(), err))
            })?;
        Ok(())
    }

    async fn video_pre_process(task: Task) {
        let source_path = Path::new(".").join("SavedFile").join(&task.media_filename);
        let destination_path = Path::new(".").join("PreProcessing").join(&task.media_filename);
        let create_folder = destination_path.clone().with_extension("");
        if let Err(entry) = Self::extract_pre_processing(&source_path, &destination_path, &create_folder).await {
            task.panic(entry.message.clone()).await;
            logging_entry!(entry);
            return;
        }
        let video_path = destination_path;
        if let Err(entry) = Self::extract_video_info(&video_path).await {
            task.panic(entry.message.clone()).await;
            logging_entry!(entry);
            return;
        }
        let result = spawn_blocking(move || {
            Self::extract_video(video_path)
        }).await;
        Self::process_extract_result(task, create_folder, result).await;
    }

    async fn extract_video_info(video_path: &PathBuf) -> Result<(), LogEntry> {
        let absolute_path = video_path.canonicalize()
            .map_err(|err| error_entry!("File Manager", "Unable to get absolute path", format!("Err:{err}")))?;
        let clean_path = absolute_path.to_string_lossy().trim_start_matches(r"\\?\").replace("\\", "/");
        let discoverer = Discoverer::new(gstreamer::ClockTime::from_seconds(5))
            .map_err(|err| error_entry!("File Manager", "Unable to create instance", format!("Err: {}", err)))?;
        let info = discoverer.discover_uri(&*format!("file:///{clean_path}"))
            .map_err(|err| error_entry!("File Manager", "Unable to create instance", format!("Err: {}", err)))?;
        let mut video_info = VideoInfo::default();
        if let Some(stream) = info.video_streams().get(0) {
            video_info.bitrate = stream.bitrate();
            if let Some(caps) = stream.caps() {
                if let Some(structure) = caps.structure(0) {
                    video_info.format = structure.name().to_string();
                    for field in structure.fields() {
                        match field.as_str() {
                            "framerate" => video_info.framerate = structure.get::<gstreamer::Fraction>(field)
                                .map_or_else(|_| "30/1".to_string(), |f| format!("{number}/{denom}", number = f.numer(), denom = f.denom())),
                            _ => {}
                        }
                    }
                }
            }
        }
        let toml_path = video_path.with_extension("toml");
        let toml_string = toml::to_string(&video_info)
            .map_err(|err| error_entry!("File Manager", "Unable to serialize data", format!("Err: {}", err)))?;
        fs::write(&toml_path, toml_string).await
            .map_err(|err| error_entry!("File Manager", "Unable to write file", format!("Err: {}", err)))?;
        Ok(())
    }

    fn extract_video(video_path: PathBuf) -> Result<(), LogEntry> {
        let saved_path = video_path.clone().with_extension("").to_path_buf();
        let pipeline_string = format!("filesrc location={:?} ! decodebin ! videoconvert ! pngenc ! multifilesink location={:?}", video_path, saved_path.join("%010d.png"));
        let pipeline = gstreamer::parse::launch(&pipeline_string)
            .map_err(|err| error_entry!("File Manager", "Unable to create instance", format!("Err: {}", err)))?;
        let bus = pipeline.bus().ok_or(error_entry!("File Manager", "Unable to create instance"))?;
        pipeline.set_state(gstreamer::State::Playing)
            .map_err(|err| error_entry!("File Manager", "Unable to set pipeline status", format!("Err: {}", err)))?;
        for message in bus.iter_timed(gstreamer::ClockTime::NONE) {
            match message.view() {
                gstreamer::MessageView::Eos(..) => break,
                gstreamer::MessageView::Error(_) => {
                    pipeline.set_state(gstreamer::State::Null)
                        .map_err(|err| error_entry!("File Manager", "無法設定管線狀態", format!("Err: {}", err)))?;
                    return if let Some(source) = message.src() {
                        let err = source.path_string();
                        Err(error_entry!("File Manager", "GStreamer internal error", format!("Err: {}", err)))
                    } else {
                        Err(error_entry!("File Manager", "GStreamer internal error"))
                    };
                }
                _ => {}
            }
        }
        pipeline.set_state(gstreamer::State::Null)
            .map_err(|err| error_entry!("File Manager", "Unable to set pipeline status", format!("Err: {}", err)))?;
        Ok(())
    }

    async fn zip_pre_process(task: Task) {
        let source_path = Path::new(".").join("SavedFile").join(&task.media_filename);
        let destination_path = Path::new(".").join("PreProcessing").join(&task.media_filename);
        let create_folder = destination_path.clone().with_extension("");
        if let Err(entry) = Self::extract_pre_processing(&source_path, &destination_path, &create_folder).await {
            task.panic(entry.message.clone()).await;
            logging_entry!(entry);
            return;
        }
        let zip_path = destination_path;
        let result = spawn_blocking(move || {
            Self::extract_zip(&zip_path)
        }).await;
        Self::process_extract_result(task, create_folder, result).await;
    }

    fn extract_zip(zip_path: &PathBuf) -> Result<(), LogEntry> {
        let allowed_extensions = ["png", "jpg", "jpeg"];
        let reader = File::open(zip_path)
            .map_err(|err| error_entry!("File Manager", "Unable to read file", format!("Err: {}", err)))?;
        let mut archive = ZipArchive::new(reader)
            .map_err(|err| error_entry!("File Manager", "Unable to create instance", format!("Err: {}", err)))?;
        let output_folder = zip_path.clone().with_extension("").to_path_buf();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|err| error_entry!("File Manager", "An error occurred while reading the file", format!("Err: {}", err)))?;
            if let Some(enclosed_path) = file.enclosed_name() {
                if let Some(extension) = enclosed_path.extension() {
                    if allowed_extensions.contains(&extension.to_str().unwrap_or("")) {
                        let output_path = output_folder.join(enclosed_path.file_name().unwrap_or_default());
                        let mut output_file = File::create(&output_path)
                            .map_err(|err| error_entry!("File Manager", "Unable to create file", format!("Err: {}", err)))?;
                        std::io::copy(&mut file, &mut output_file)
                            .map_err(|err| error_entry!("File Manager", "Unable to write file", format!("Err: {}", err)))?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn process_extract_result(mut task: Task, created_folder: PathBuf, result: Result<Result<(), LogEntry>, JoinError>) {
        match result {
            Ok(Ok(_)) => {
                match Self::file_count(&created_folder).await {
                    Ok(count) => {
                        task.update_unprocessed(count).await;
                        Self::forward_to_task_manager(task).await;
                    },
                    Err(entry) => {
                        task.panic("Unable to read folder".to_string()).await;
                        logging_entry!(entry);
                    },
                }
            },
            Ok(Err(entry)) => {
                task.panic(entry.message.clone()).await;
                logging_entry!(entry);
            },
            Err(err) => {
                task.panic("Panic occurs during execution".to_string()).await;
                logging_critical!("File Manager", "Panic occurs during execution", format!("Err: {}", err));
            },
        }
    }

    async fn draw_bounding_box(image_task: &ImageTask, config: &Config, font: &Font<'_>) -> Result<RgbImage, LogEntry> {
        let border_color = Rgb(config.border_color);
        let text_color = Rgb(config.text_color);
        let image_path = image_task.image_filepath.clone();
        let mut image = image::open(&image_path)
            .map_err(|err| error_entry!("File Manager", "Unable to read file", format!("Err: {}", err)))?
            .to_rgb8();
        for bounding_box in &image_task.bounding_boxes {
            let base_rectangle = Rect::at(bounding_box.xmin as i32, bounding_box.ymin as i32).of_size(bounding_box.xmax - bounding_box.xmin, bounding_box.ymax - bounding_box.ymin);
            for i in 0..config.border_width {
                let offset_rect = Rect::at(base_rectangle.left() - i as i32, base_rectangle.top() - i as i32).of_size(base_rectangle.width() + 2 * i, base_rectangle.height() + 2 * i);
                draw_hollow_rect_mut(&mut image, offset_rect, border_color);
            }
            let scale = Scale::uniform(config.font_size);
            let text = format!("{name}: {confidence:.2}%", name = bounding_box.name, confidence = bounding_box.confidence);
            let position_x = bounding_box.xmin as i32;
            let position_y = (bounding_box.ymax + config.border_width + 10) as i32;
            draw_text_mut(&mut image, text_color, position_x, position_y, scale, &font, &text);
        }
        Ok(image)
    }

    async fn picture_post_processing(task: Task) {
        if let Some(image_task) = task.result.get(0) {
            let config = Config::now().await;
            let font_path = &config.font_path;
            match fs::read(font_path).await {
                Ok(font_data) => {
                    if let Some(font) = Font::try_from_bytes(&font_data) {
                        let saved_path = Path::new(".").join("PostProcessing").join(image_task.image_filename.clone());
                        match Self::draw_bounding_box(image_task, &config, &font).await {
                            Ok(image) => {
                                match image.save(&saved_path) {
                                    Ok(_) => Self::forward_to_repository(task).await,
                                    Err(err) => {
                                        task.panic("Unable to write file".to_string()).await;
                                        logging_error!("File Manager", "Unable to write file", format!("Err: {}", err));
                                    },
                                }
                            },
                            Err(entry) => {
                                task.panic(entry.message.clone()).await;
                                logging_entry!(entry);
                            },
                        }
                    } else {
                        task.panic("Unable to parse data in file".to_string()).await;
                        logging_error!("File Manager", "Unable to parse data in file");
                    }
                },
                Err(err) => {
                    let error_message = "Unable to read file".to_string();
                    task.panic(error_message).await;
                    logging_error!("File Manager", "Unable to read file", format!("Err: {}", err));
                },
            }
        } else {
            task.panic("Missing tasks".to_string()).await;
            logging_error!("File Manager", "Missing tasks");
        }
    }

    async fn recombination_pre_processing(task: &mut Task, create_folder: &PathBuf) -> Result<(), LogEntry> {
        fs::create_dir(&create_folder).await
            .map_err(|err| error_entry!("File Manager", format!("Cannot create folder {}", create_folder.display()), format!("Err: {err}")))?;
        let config = Config::now().await;
        let font_data = fs::read(&config.font_path).await
            .map_err(|err| error_entry!("File Manager", "Unable to read file", format!("Err: {}", err)))?;
        let font = Font::try_from_bytes(&font_data)
            .ok_or(error_entry!("File Manager", "Unable to parse data in file"))?;
        for image_task in &task.result {
            let saved_path = create_folder.join(image_task.image_filename.clone());
            let image = Self::draw_bounding_box(image_task, &config, &font).await?;
            image.save(&saved_path)
                .map_err(|err| error_entry!("File Manager", "Unable to write file", format!("Err: {err}")))?;
        }
        Ok(())
    }

    async fn video_post_processing(mut task: Task) {
        let video_info_path = Path::new(".").join("PreProcessing").join(task.media_filename.clone()).with_extension("toml");
        let target_path = Path::new(".").join("PostProcessing").join(task.media_filename.clone());
        let create_folder = target_path.with_extension("");
        if let Err(entry) = Self::recombination_pre_processing(&mut task, &create_folder).await {
            task.panic(entry.message.clone()).await;
            logging_entry!(entry);
            return;
        }
        let result = spawn_blocking(move || {
            Self::recombination_video(video_info_path, create_folder, target_path)
        }).await;
        Self::process_recombination_result(task, result).await;
    }

    fn recombination_video(video_info_path: PathBuf, frame_folder: PathBuf, target_path: PathBuf) -> Result<(), LogEntry> {
        let toml_str = std::fs::read_to_string(&video_info_path)
            .map_err(|err| error_entry!("File Manager", format!("Unable to read file {}", video_info_path.display()), format!("Err: {}", err)))?;
        let video_info: VideoInfo = toml::from_str(&toml_str).unwrap_or_default();
        let bitrate = video_info.bitrate;
        let encoder = match video_info.format.as_str() {
            "video/x-h265" => format!("x265enc bitrate={}", bitrate),
            "video/x-h264" => format!("x264enc bitrate={}", bitrate),
            "video/x-vp9" => format!("vp9enc target-bitrate={}", bitrate),
            "video/x-vp8" => format!("vp8enc target-bitrate={}", bitrate),
            _ => format!("x265enc bitrate={}", bitrate),
        };
        let muxer = match target_path.extension().and_then(OsStr::to_str) {
            Some("mp4") => "mp4mux",
            Some("avi") => "avimux",
            Some("mkv") => "matroskamux",
            _ => "mp4mux",
        };
        let pipeline_string = format!("multifilesrc location={:?} index=1 caps=image/png,framerate=(fraction){} ! pngdec ! videoconvert ! {} ! {} ! filesink location={:?}", frame_folder.join("%010d.png"), video_info.framerate, encoder, muxer, target_path);
        let pipeline = gstreamer::parse::launch(&pipeline_string)
            .map_err(|err| error_entry!("File Manager", "Unable to create instance", format!("Err: {}", err)))?;
        let bus = pipeline.bus().ok_or(error_entry!("File Manager", "Unable to create instance"))?;
        pipeline.set_state(gstreamer::State::Playing)
            .map_err(|err| error_entry!("File Manager", "Unable to set pipeline status", format!("Err: {}", err)))?;
        for message in bus.iter_timed(gstreamer::ClockTime::NONE) {
            match message.view() {
                gstreamer::MessageView::Eos(..) => break,
                gstreamer::MessageView::Error(_) => {
                    pipeline.set_state(gstreamer::State::Null)
                        .map_err(|err| error_entry!("File Manager", "Unable to set pipeline status", format!("Err: {}", err)))?;
                    return if let Some(source) = message.src() {
                        let err = source.path_string();
                        Err(error_entry!("File Manager", "GStreamer internal error", format!("Err: {}", err)))
                    } else {
                        Err(error_entry!("File Manager", "GStreamer internal error"))
                    };
                }
                _ => {}
            }
        }
        pipeline.set_state(gstreamer::State::Null)
            .map_err(|err| error_entry!("File Manager", "Unable to set pipeline status", format!("Err: {}", err)))?;
        Ok(())
    }

    async fn zip_post_processing(mut task: Task) {
        let target_path = Path::new(".").join("PostProcessing").join(task.media_filename.clone());
        let create_folder = target_path.with_extension("");
        if let Err(entry) = Self::recombination_pre_processing(&mut task, &create_folder).await {
            task.panic(entry.message.clone()).await;
            logging_entry!(entry);
            return;
        }
        let result = spawn_blocking(move || {
            Self::recombination_zip(create_folder, target_path)
        }).await;
        Self::process_recombination_result(task, result).await;
    }

    fn recombination_zip(source_folder: PathBuf, target_path: PathBuf) -> Result<(), LogEntry> {
        let file = File::create(&target_path).map_err(|err| error_entry!("File Manager", "Unable to create file", format!("Err: {}", err)))?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for entry in std::fs::read_dir(&source_folder)
            .map_err(|err| error_entry!("File Manager", "Unable to read folder", format!("Err: {}", err)))?
        {
            let entry = entry.map_err(|err|
                error_entry!("File Manager", "An error occurred while reading the folder", format!("Err: {}", err)))?;
            let path = entry.path();
            let file_name = path.file_name().ok_or(
                error_entry!("File Manager", "Invalid file")
            )?.to_string_lossy();
            zip.start_file(file_name, options)
                .map_err(|err| error_entry!("File Manager", "Unable to create file", format!("Err: {}", err)))?;
            let mut file_contents = Vec::new();
            File::open(&path)
                .map_err(|err| error_entry!("File Manager", "Unable to read file", format!("Err: {}", err)))?
                .read_to_end(&mut file_contents)
                .map_err(|err| error_entry!("File Manager", "An error occurred while reading the file", format!("Err: {}", err)))?;
            zip.write_all(&file_contents)
                .map_err(|err| error_entry!("File Manager", "An error occurred while writing the file", format!("Err: {}", err)))?;
        }
        zip.finish()
            .map_err(|err| error_entry!("File Manager", "An error occurred while writing the file", format!("Err: {}", err)))?;
        Ok(())
    }

    async fn process_recombination_result(task: Task, result: Result<Result<(), LogEntry>, JoinError>) {
        match result {
            Ok(Ok(_)) => Self::forward_to_repository(task).await,
            Ok(Err(entry)) => {
                task.panic(entry.message.clone()).await;
                logging_entry!(entry);
            }
            Err(err) => {
                task.panic("Panic occurs during execution".to_string()).await;
                logging_critical!("File Manager", "Panic occurs during execution", format!("Err: {}", err));
            }
        }
    }

    async fn file_count(path: &PathBuf) -> Result<usize, LogEntry> {
        let read_dir = fs::read_dir(path).await
            .map_err(|err| error_entry!("File Manager", "Unable to read folder", format!("Err: {}", err)))?;
        let dir_entries = ReadDirStream::new(read_dir);
        let count = dir_entries.filter_map(|entry| async {
            entry.ok().and_then(|e| if e.path().is_file() { Some(()) } else { None })
        }).count().await;
        Ok(count)
    }

    async fn forward_to_task_manager(mut task: Task) {
        task.change_status(TaskStatus::Waiting);
        TaskManager::add_task(task).await;
    }

    async fn forward_to_repository(mut task: Task) {
        task.change_status(TaskStatus::Success);
        ResultRepository::task_success(task).await
    }
}
