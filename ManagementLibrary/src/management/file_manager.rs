use crate::management::result_repository::ResultRepository;
use crate::management::task_manager::TaskManager;
use crate::management::utils::image_task::ImageTask;
use crate::management::utils::task::{Task, TaskStatus};
use crate::management::utils::video_info::VideoInfo;
use crate::utils::config::Config;
use crate::utils::logging::*;
use ab_glyph::{FontVec, PxScale};
use futures::StreamExt;
use gstreamer::prelude::*;
use gstreamer_pbutils::prelude::*;
use gstreamer_pbutils::Discoverer;
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::task::{spawn_blocking, JoinError, JoinHandle};
use tokio::time::sleep;
use tokio_stream::wrappers::ReadDirStream;
use zip::read::ZipArchive;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

lazy_static! {
    static ref FILE_MANAGER: RwLock<FileManager> = RwLock::new(FileManager::new());
}

pub struct FileManager {
    pre_process_tasks: VecDeque<Task>,
    post_process_tasks: VecDeque<Task>,
    join_handles: Vec<JoinHandle<()>>,
    terminate: bool,
    cancel_flag: Arc<AtomicBool>,
}

impl FileManager {
    fn new() -> Self {
        Self {
            pre_process_tasks: VecDeque::new(),
            post_process_tasks: VecDeque::new(),
            join_handles: Vec::new(),
            terminate: false,
            cancel_flag: Arc::new(AtomicBool::new(false)),
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
        let pre_processing_handle = tokio::spawn(async {
            Self::pre_processing().await
        });
        let post_processing_handle = tokio::spawn(async {
            Self::post_processing().await
        });
        Self::add_join_handle(pre_processing_handle).await;
        Self::add_join_handle(post_processing_handle).await;
        logging_information!("File Manager", "Online now");
    }

    async fn initialize() {
        logging_information!("File Manager", "Initializing");
        let folders = ["SavedModel", "SavedFile", "PreProcessing", "PostProcessing", "Result"];
        for &folder_name in &folders {
            match fs::create_dir(folder_name).await {
                Ok(_) => logging_information!("File Manager", format!("Creating {folder_name} folder successfully")),
                Err(err) => logging_critical!("File Manager", format!("Cannot create folder {folder_name}"), format!("Err: {err}")),
            }
        }
        match gstreamer::init() {
            Ok(_) => logging_information!("File Manager", "GStreamer initialization successful"),
            Err(err) => logging_critical!("File Manager", "GStreamer initialization failed", format!("Err: {err}")),
        }
        logging_information!("File Manager", "Initialization completed");
    }

    pub async fn terminate() {
        logging_information!("File Manager", "Termination in progress");
        let handles = {
            let mut instance = Self::instance_mut().await;
            instance.terminate = true;
            instance.cancel_flag.store(true, Ordering::Relaxed);
            std::mem::take(&mut instance.join_handles)
        };
        for handle in handles {
            if let Err(err) = handle.await {
                logging_error!("File Manager", "Task panicked during termination", format!("Err: {err}"));
            }
        }
        Self::cleanup().await;
        logging_information!("File Manager", "Termination complete");
    }

    async fn cleanup() {
        logging_information!("File Manager", "Cleaning up");
        let folders = ["SavedModel", "SavedFile", "PreProcessing", "PostProcessing", "Result"];
        for &folder_name in &folders {
            match fs::remove_dir_all(folder_name).await {
                Ok(_) => logging_information!("File Manager", format!("Delete {folder_name} folder successfully")),
                Err(err) => logging_error!("File Manager", format!("Failed to delete {folder_name} folder"), format!("Err: {err}")),
            }
        };
        logging_information!("File Manager", "Cleanup completed");
    }

    async fn add_join_handle(join_handle: JoinHandle<()>) {
        Self::instance_mut().await.join_handles.push(join_handle);
    }

    pub async fn add_pre_process_task(task: Task) {
        Self::instance_mut().await.pre_process_tasks.push_back(task);
    }

    pub async fn add_post_process_task(task: Task) {
        Self::instance_mut().await.post_process_tasks.push_back(task);
    }

    async fn pre_processing() {
        let config = Config::now().await;
        while !Self::instance().await.terminate {
            let task = Self::instance_mut().await.pre_process_tasks.pop_front();
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
            let task = Self::instance_mut().await.post_process_tasks.pop_front();
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
                logging_error!("File Manager", "Unable to move file", format!("Source: {}, Destination: {}, Err: {}", source_path.display(), destination_path.display(), err));
            }
        }
    }

    async fn prepare_pre_processing(source_path: &PathBuf, destination_path: &PathBuf, create_folder: &PathBuf) -> Result<(), LogEntry> {
        fs::create_dir(&create_folder).await
            .map_err(|err|
                error_entry!("File Manager", "Unable to create folder",format!("Folder:{}, Err: {}", create_folder.display(), err)))?;
        fs::rename(&source_path, &destination_path).await
            .map_err(|err|
                error_entry!("File Manager", "Unable to move file",format!("Source: {}, Destination: {}, Err: {}", source_path.display(), destination_path.display(), err)))?;
        Ok(())
    }

    async fn video_pre_process(task: Task) {
        let source_path = Path::new(".").join("SavedFile").join(&task.media_filename);
        let destination_path = Path::new(".").join("PreProcessing").join(&task.media_filename);
        let create_folder = destination_path.clone().with_extension("");
        if let Err(entry) = Self::prepare_pre_processing(&source_path, &destination_path, &create_folder).await {
            task.panic(entry.message.clone()).await;
            logging_entry!(entry);
            return;
        }
        let video_path = destination_path;
        let extract_folder = create_folder;
        if let Err(entry) = Self::fetch_video_info(&video_path).await {
            task.panic(entry.message.clone()).await;
            logging_entry!(entry);
            return;
        }
        let cancel_flag = Self::instance().await.cancel_flag.clone();
        let result = spawn_blocking(move || {
            Self::split_video_into_frames(video_path, cancel_flag)
        }).await;
        Self::handle_pre_process_result(task, extract_folder, result).await;
    }

    async fn fetch_video_info(video_path: &PathBuf) -> Result<(), LogEntry> {
        let absolute_path = video_path.canonicalize()
            .map_err(|err| error_entry!("File Manager", "Unable to get absolute path", format!("Path: {}, Err: {}", video_path.display(), err)))?;
        let clean_path = absolute_path.to_string_lossy().trim_start_matches(r"\\?\").replace("\\", "/");
        let discoverer = Discoverer::new(gstreamer::ClockTime::from_seconds(5))
            .map_err(|err| error_entry!("File Manager", "Unable to create instance", format!("Err: {err}")))?;
        let info = discoverer.discover_uri(&*format!("file:///{clean_path}"))
            .map_err(|err| error_entry!("File Manager", "Unable to create instance", format!("Err: {err}")))?;
        let mut video_info = VideoInfo::default();
        if let Some(stream) = info.video_streams().get(0) {
            video_info.bitrate = stream.bitrate();
            if let Some(caps) = stream.caps() {
                if let Some(structure) = caps.structure(0) {
                    video_info.format = structure.name().to_string();
                    for field in structure.fields() {
                        match field.as_str() {
                            "framerate" => video_info.framerate = structure.get::<gstreamer::Fraction>(field)
                                .map_or_else(|_| "30/1".to_string(), |f| format!("{}/{}", f.numer(), f.denom())),
                            _ => {}
                        }
                    }
                }
            }
        }
        let toml_path = video_path.with_extension("toml");
        let video_info = toml::to_string(&video_info)
            .map_err(|err| error_entry!("File Manager", "Unable to serialize data", format!("Err: {err}")))?;
        fs::write(&toml_path, video_info).await
            .map_err(|err| error_entry!("File Manager", "Unable to write file", format!("File:{}, Err: {}", toml_path.display(), err)))?;
        Ok(())
    }

    fn split_video_into_frames(video_path: PathBuf, cancel_flag: Arc<AtomicBool>) -> Result<(), LogEntry> {
        let config = Config::now_blocking();
        let saved_path = video_path.clone().with_extension("").to_path_buf();
        let pipeline_string = format!("filesrc location={:?} ! decodebin ! videoconvert ! pngenc ! multifilesink location={:?}", video_path, saved_path.join("%010d.png"));
        let pipeline = gstreamer::parse::launch(&pipeline_string)
            .map_err(|err| error_entry!("File Manager", "Unable to create instance", format!("Err: {err}")))?;
        let bus = pipeline.bus().ok_or(error_entry!("File Manager", "Unable to create instance"))?;
        pipeline.set_state(gstreamer::State::Playing)
            .map_err(|err| error_entry!("File Manager", "Unable to set pipeline status", format!("Err: {err}")))?;
        let polling_interval = gstreamer::ClockTime::from_mseconds(config.polling_interval);
        let result = loop {
            if cancel_flag.load(Ordering::Relaxed) {
                break Err(information_entry!("File Manager", "Operation cancelled"));
            }
            if let Some(message) = bus.timed_pop(polling_interval) {
                match message.view() {
                    gstreamer::MessageView::Eos(..) => break Ok(()),
                    gstreamer::MessageView::Error(err) => break Err(error_entry!("File Manager", format!("GStreamer internal error: {}", err.error()))),
                    _ => {}
                }
            }
        };
        pipeline.set_state(gstreamer::State::Null)
            .map_err(|err| error_entry!("File Manager", "Unable to set pipeline status", format!("Err: {err}")))?;
        result
    }

    fn split_video_into_clips() {

    }

    async fn zip_pre_process(task: Task) {
        let source_path = Path::new(".").join("SavedFile").join(&task.media_filename);
        let destination_path = Path::new(".").join("PreProcessing").join(&task.media_filename);
        let create_folder = destination_path.clone().with_extension("");
        if let Err(entry) = Self::prepare_pre_processing(&source_path, &destination_path, &create_folder).await {
            task.panic(entry.message.clone()).await;
            logging_entry!(entry);
            return;
        }
        let zip_path = destination_path;
        let cancel_flag = Self::instance().await.cancel_flag.clone();
        let result = spawn_blocking(move || {
            Self::unzip(&zip_path, cancel_flag)
        }).await;
        Self::handle_pre_process_result(task, create_folder, result).await;
    }

    fn unzip(zip_path: &PathBuf, cancel_flag: Arc<AtomicBool>) -> Result<(), LogEntry> {
        let allowed_extensions = ["png", "jpg", "jpeg"];
        let reader = File::open(zip_path)
            .map_err(|err| error_entry!("File Manager", "Unable to read file", format!("File:{}, Err: {}", zip_path.display(), err)))?;
        let mut archive = ZipArchive::new(reader)
            .map_err(|err| error_entry!("File Manager", "Unable to create instance", format!("Err: {err}")))?;
        let extract_folder = zip_path.clone().with_extension("").to_path_buf();
        for i in 0..archive.len() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(information_entry!("File Manager", "Operation cancelled"));
            }
            let mut file = archive.by_index(i)
                .map_err(|err| error_entry!("File Manager", "An error occurred while reading the file", format!("File: {}, Err: {}", zip_path.display(), err)))?;
            if let Some(enclosed_path) = file.enclosed_name() {
                if let Some(extension) = enclosed_path.extension() {
                    if allowed_extensions.contains(&extension.to_str().unwrap_or("")) {
                        let output_path = extract_folder.join(enclosed_path.file_name().unwrap_or_default());
                        let mut output_file = File::create(&output_path)
                            .map_err(|err| error_entry!("File Manager", "Unable to create file", format!("File: {}, Err: {}", output_path.display(), err)))?;
                        std::io::copy(&mut file, &mut output_file)
                            .map_err(|err| error_entry!("File Manager", "Unable to write file", format!("File: {}, Err: {}", output_path.display(), err)))?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_pre_process_result(mut task: Task, created_folder: PathBuf, result: Result<Result<(), LogEntry>, JoinError>) {
        match result {
            Ok(Ok(_)) => {
                match Self::file_count(&created_folder).await {
                    Ok(count) => {
                        task.update_unprocessed(count).await;
                        Self::forward_to_task_manager(task).await;
                    }
                    Err(entry) => {
                        task.panic("Unable to read folder".to_string()).await;
                        logging_entry!(entry);
                    }
                }
            }
            Ok(Err(entry)) => {
                task.panic(entry.message.clone()).await;
                logging_entry!(entry);
            }
            Err(err) => {
                task.panic("Panic occurs during execution".to_string()).await;
                logging_error!("File Manager", "Panic occurs during execution", format!("Err: {err}"));
            }
        }
    }

    async fn draw_bounding_box(image_task: &ImageTask, config: &Config, font: &FontVec) -> Result<RgbImage, LogEntry> {
        let border_color = Rgb(config.border_color);
        let text_color = Rgb(config.text_color);
        let image_path = image_task.image_filepath.clone();
        let mut image = image::open(&image_path)
            .map_err(|err| error_entry!("File Manager", "Unable to read file", format!("File: {}, Err: {}", image_path.display(), err)))?
            .to_rgb8();
        for bounding_box in &image_task.bounding_boxes {
            let base_rectangle = Rect::at(bounding_box.xmin as i32, bounding_box.ymin as i32).of_size(bounding_box.xmax - bounding_box.xmin, bounding_box.ymax - bounding_box.ymin);
            for i in 0..config.border_width {
                let offset_rect = Rect::at(base_rectangle.left() - i as i32, base_rectangle.top() - i as i32).of_size(base_rectangle.width() + 2 * i, base_rectangle.height() + 2 * i);
                draw_hollow_rect_mut(&mut image, offset_rect, border_color);
            }
            let scale = PxScale::from(config.font_size);
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
                    match FontVec::try_from_vec(font_data) {
                        Ok(font) => {
                            let saved_path = Path::new(".").join("PostProcessing").join(image_task.image_filename.clone());
                            match Self::draw_bounding_box(image_task, &config, &font).await {
                                Ok(image) => {
                                    if let Err(err) = image.save(&saved_path) {
                                        task.panic("Unable to write file".to_string()).await;
                                        logging_error!("File Manager", "Unable to write file", format!("File: {}, Err: {}", saved_path.display(), err));
                                    } else {
                                        Self::forward_to_repository(task).await;
                                    }
                                }
                                Err(entry) => {
                                    task.panic(entry.message.clone()).await;
                                    logging_entry!(entry);
                                }
                            }
                        }
                        Err(err) => {
                            task.panic("Unable to parse data in file".to_string()).await;
                            logging_error!("File Manager", "Unable to parse data in file", format!("File: {}, Err: {}", font_path, err));
                        }
                    }
                }
                Err(err) => {
                    let error_message = "Unable to read file".to_string();
                    task.panic(error_message).await;
                    logging_error!("File Manager", "Unable to read file", format!("File: {}, Err: {}", font_path, err));
                }
            }
        } else {
            task.panic("Missing tasks".to_string()).await;
            logging_error!("File Manager", "Missing tasks");
        }
    }

    async fn prepare_post_processing(task: &mut Task, create_folder: &PathBuf) -> Result<(), LogEntry> {
        fs::create_dir(&create_folder).await
            .map_err(|err| error_entry!("File Manager", "Cannot create folder", format!("Folder: {}, Err: {}", create_folder.display(), err)))?;
        let config = Config::now().await;
        let font_path = &config.font_path;
        let font_data = fs::read(font_path).await
            .map_err(|err| error_entry!("File Manager", "Unable to read file", format!("File: {}, Err: {}", font_path, err)))?;
        let font = FontVec::try_from_vec(font_data)
            .map_err(|err| error_entry!("File Manager", "Unable to parse data in file", format!("File: {}, Err: {}", font_path, err)))?;
        for image_task in &task.result {
            let saved_path = create_folder.join(image_task.image_filename.clone());
            let image = Self::draw_bounding_box(image_task, &config, &font).await?;
            image.save(&saved_path)
                .map_err(|err| error_entry!("File Manager", "Unable to write file", format!("File: {}, Err: {}", saved_path.display(), err)))?;
        }
        Ok(())
    }

    async fn video_post_processing(mut task: Task) {
        let video_info_path = Path::new(".").join("PreProcessing").join(task.media_filename.clone()).with_extension("toml");
        let target_path = Path::new(".").join("PostProcessing").join(task.media_filename.clone());
        let create_folder = target_path.with_extension("");
        if let Err(entry) = Self::prepare_post_processing(&mut task, &create_folder).await {
            task.panic(entry.message.clone()).await;
            logging_entry!(entry);
            return;
        }
        let cancel_flag = Self::instance().await.cancel_flag.clone();
        let result = spawn_blocking(move || {
            Self::recombination_video_from_frame(video_info_path, create_folder, target_path, cancel_flag)
        }).await;
        Self::handle_post_process_result(task, result).await;
    }

    fn recombination_video_from_frame(video_info_path: PathBuf, frame_folder: PathBuf, saved_path: PathBuf, cancel_flag: Arc<AtomicBool>) -> Result<(), LogEntry> {
        let config = Config::now_blocking();
        let toml_str = std::fs::read_to_string(&video_info_path)
            .map_err(|err| error_entry!("File Manager", "Unable to read file", format!("File: {}, Err: {}", video_info_path.display(), err)))?;
        let video_info: VideoInfo = toml::from_str(&toml_str).unwrap_or_default();
        let bitrate = video_info.bitrate;
        let encoder = match video_info.format.as_str() {
            "video/x-h265" => format!("x265enc bitrate={}", bitrate),
            "video/x-h264" => format!("x264enc bitrate={}", bitrate),
            "video/x-vp9" => format!("vp9enc target-bitrate={}", bitrate),
            "video/x-vp8" => format!("vp8enc target-bitrate={}", bitrate),
            _ => format!("x265enc bitrate={}", bitrate),
        };
        let muxer = match saved_path.extension().and_then(OsStr::to_str) {
            Some("mp4") => "mp4mux",
            Some("avi") => "avimux",
            Some("mkv") => "matroskamux",
            _ => "mp4mux",
        };
        let pipeline_string = format!("multifilesrc location={:?} index=1 caps=image/png,framerate=(fraction){} ! pngdec ! videoconvert ! {} ! {} \
            ! filesink location={:?}", frame_folder.join("%010d.png"), video_info.framerate, encoder, muxer, saved_path);
        let pipeline = gstreamer::parse::launch(&pipeline_string)
            .map_err(|err| error_entry!("File Manager", "Unable to create instance", format!("Err: {err}")))?;
        let bus = pipeline.bus().ok_or(error_entry!("File Manager", "Unable to create instance"))?;
        pipeline.set_state(gstreamer::State::Playing)
            .map_err(|err| error_entry!("File Manager", "Unable to set pipeline status", format!("Err: {err}")))?;
        let polling_interval = gstreamer::ClockTime::from_mseconds(config.polling_interval);
        let result = loop {
            if cancel_flag.load(Ordering::Relaxed) {
                break Err(information_entry!("File Manager", "Operation cancelled"));
            }
            if let Some(message) = bus.timed_pop(polling_interval) {
                match message.view() {
                    gstreamer::MessageView::Eos(..) => break Ok(()),
                    gstreamer::MessageView::Error(err) => break Err(error_entry!("File Manager", format!("GStreamer internal error: {}", err.error()))),
                    _ => {}
                }
            }
        };
        pipeline.set_state(gstreamer::State::Null)
            .map_err(|err| error_entry!("File Manager", "Unable to set pipeline status", format!("Err: {err}")))?;
        result
    }

    async fn zip_post_processing(mut task: Task) {
        let target_path = Path::new(".").join("PostProcessing").join(task.media_filename.clone());
        let create_folder = target_path.with_extension("");
        if let Err(entry) = Self::prepare_post_processing(&mut task, &create_folder).await {
            task.panic(entry.message.clone()).await;
            logging_entry!(entry);
            return;
        }
        let cancel_flag = Self::instance().await.cancel_flag.clone();
        let result = spawn_blocking(move || {
            Self::recombination_zip(create_folder, target_path, cancel_flag)
        }).await;
        Self::handle_post_process_result(task, result).await;
    }

    fn recombination_zip(source_folder: PathBuf, target_path: PathBuf, cancel_flag: Arc<AtomicBool>) -> Result<(), LogEntry> {
        let file = File::create(&target_path)
            .map_err(|err| error_entry!("File Manager", "Unable to create file", format!("File: {}, Err: {}", target_path.display(), err)))?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for entry in std::fs::read_dir(&source_folder)
            .map_err(|err| error_entry!("File Manager", "Unable to read folder", format!("Folder: {}, Err: {}", source_folder.display(), err)))?
        {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(information_entry!("File Manager", "Operation cancelled"));
            }
            let entry = entry.map_err(|err|
                error_entry!("File Manager", "An error occurred while reading the folder", format!("Folder: {}, Err: {}", source_folder.display(), err)))?;
            let path = entry.path();
            let file_name = path.file_name().ok_or(error_entry!("File Manager", "Invalid file"))?.to_string_lossy();
            zip.start_file(file_name.clone(), options)
                .map_err(|err| error_entry!("File Manager", "Unable to create file", format!("File: {}, Err: {}", file_name.clone(), err)))?;
            let mut file_contents = Vec::new();
            File::open(&path)
                .map_err(|err| error_entry!("File Manager", "Unable to read file", format!("File: {}, Err: {}", path.display(), err)))?
                .read_to_end(&mut file_contents)
                .map_err(|err| error_entry!("File Manager", "An error occurred while reading the file", format!("File: {}, Err: {}", path.display(), err)))?;
            zip.write_all(&file_contents)
                .map_err(|err| error_entry!("File Manager", "An error occurred while writing the file", format!("File: {}, Err: {}", file_name, err)))?;
        }
        zip.finish()
            .map_err(|err| error_entry!("File Manager", "An error occurred while writing the file", format!("File: {}, Err: {}", target_path.display(), err)))?;
        Ok(())
    }

    async fn handle_post_process_result(task: Task, result: Result<Result<(), LogEntry>, JoinError>) {
        match result {
            Ok(Ok(_)) => Self::forward_to_repository(task).await,
            Ok(Err(entry)) => {
                task.panic(entry.message.clone()).await;
                logging_entry!(entry);
            }
            Err(err) => {
                task.panic("Panic occurs during execution".to_string()).await;
                logging_error!("File Manager", "Panic occurs during execution", format!("Err: {err}"));
            }
        }
    }

    async fn file_count(path: &PathBuf) -> Result<usize, LogEntry> {
        let read_dir = fs::read_dir(path).await
            .map_err(|err| error_entry!("File Manager", "Unable to read folder", format!("Folder: {}, Err: {}", path.display(), err)))?;
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
