use crate::management::task_manager::TaskManager;
use crate::management::utils::task::{Task, TaskStatus};
use crate::management::utils::video_info::VideoInfo;
use crate::utils::config::{Config, SplitMode};
use crate::utils::logging::*;
use futures::StreamExt;
use gstreamer::prelude::*;
use gstreamer_pbutils::prelude::*;
use gstreamer_pbutils::Discoverer;
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
use tokio::task::{spawn_blocking, JoinHandle};
use tokio::time::sleep;
use tokio_stream::wrappers::ReadDirStream;
use zip::read::ZipArchive;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

lazy_static! {
    static ref MEDIA_PROCESSOR: RwLock<MediaProcessor> = RwLock::new(MediaProcessor::new());
}

pub struct MediaProcessor {
    pre_process_tasks: VecDeque<Task>,
    post_process_tasks: VecDeque<Task>,
    join_handles: Vec<JoinHandle<()>>,
    terminate: bool,
    cancel_flag: Arc<AtomicBool>,
}

impl MediaProcessor {
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
        MEDIA_PROCESSOR.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        MEDIA_PROCESSOR.write().await
    }

    pub async fn run() {
        Self::initialize().await;
        let pre_process_handle = tokio::spawn(async {
            Self::pre_process().await
        });
        let post_process_handle = tokio::spawn(async {
            Self::post_process().await
        });
        Self::add_join_handle(pre_process_handle).await;
        Self::add_join_handle(post_process_handle).await;
        logging_information!(SystemEntry::Online);
    }

    async fn initialize() {
        logging_information!(SystemEntry::Initializing);
        let folders = ["SavedModel", "SavedFile", "PreProcess", "PostProcess", "Result"];
        for &folder_name in &folders {
            let path = PathBuf::from(folder_name);
            if let Err(err) = fs::create_dir(&path).await {
                logging_critical!(IOEntry::CreateDirectoryError(path.display(), err));
            }
        }
        if let Err(err) = gstreamer::init() {
            logging_critical!(GStreamerEntry::InitializeError(err));
        }
        logging_information!(SystemEntry::InitializeComplete);
    }

    pub async fn terminate() {
        logging_information!(SystemEntry::Terminating);
        let handles = {
            let mut instance = Self::instance_mut().await;
            instance.terminate = true;
            instance.cancel_flag.store(true, Ordering::Relaxed);
            std::mem::take(&mut instance.join_handles)
        };
        for handle in handles {
            if let Err(err) = handle.await {
                logging_error!(SystemEntry::TaskPanickedError(err));
            }
        }
        Self::cleanup().await;
        logging_information!(SystemEntry::TerminateComplete);
    }

    async fn cleanup() {
        logging_information!(SystemEntry::Cleaning);
        let folders = ["SavedModel", "SavedFile", "PreProcess", "PostProcess", "Result"];
        for &folder_name in &folders {
            let path = PathBuf::from(folder_name);
            if let Err(err) = fs::remove_dir_all(&path).await {
                logging_error!(IOEntry::DeleteDirectoryError(path.display(), err));
            }
        };
        logging_information!(SystemEntry::CleanComplete);
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

    async fn pre_process() {
        let config = Config::now().await;
        while !Self::instance().await.terminate {
            let task = Self::instance_mut().await.pre_process_tasks.pop_front();
            match task {
                Some(mut task) => {
                    TaskManager::change_task_status(&task.uuid, TaskStatus::PreProcessing).await;
                    let result = match Path::new(&task.media_file_name).extension().and_then(OsStr::to_str) {
                        Some("png") | Some("jpg") | Some("jpeg") => Self::picture_pre_process(&mut task).await,
                        Some("mp4") | Some("avi") | Some("mkv") => Self::video_pre_process(&mut task).await,
                        Some("zip") => Self::zip_pre_process(&mut task).await,
                        _ => Err(error_entry!(TaskEntry::UnSupportFileType(task.uuid)))
                    };
                    match result {
                        Ok(_) => TaskManager::distribute_task(task).await,
                        Err(err) => {
                            TaskManager::task_failed(&task.uuid, err.message.clone()).await;
                            logging_entry!(err);
                        }
                    }
                }
                None => sleep(Duration::from_millis(config.internal_timestamp)).await
            }
        }
    }

    async fn post_process() {
        let config = Config::now().await;
        while !Self::instance().await.terminate {
            let task = Self::instance_mut().await.post_process_tasks.pop_front();
            match task {
                Some(mut task) => {
                    TaskManager::change_task_status(&task.uuid, TaskStatus::PostProcessing).await;
                    let result = match Path::new(&task.media_file_name).extension().and_then(OsStr::to_str) {
                        Some("png") | Some("jpg") | Some("jpeg") => Self::picture_post_processing(&mut task).await,
                        Some("mp4") | Some("avi") | Some("mkv") => Self::video_post_processing(&mut task).await,
                        Some("zip") => Self::zip_post_processing(&mut task).await,
                        _ => Err(error_entry!(TaskEntry::UnSupportFileType(task.uuid)))
                    };
                    match result {
                        Ok(_) => TaskManager::task_success(&task.uuid).await,
                        Err(err) => {
                            TaskManager::task_failed(&task.uuid, err.message.clone()).await;
                            logging_entry!(err);
                        }
                    }
                }
                None => sleep(Duration::from_millis(config.internal_timestamp)).await
            }
        }
    }

    async fn picture_pre_process(task: &mut Task) -> Result<(), LogEntry> {
        let uuid = task.uuid.to_string();
        #[cfg(target_os = "linux")]
        let source_path = PathBuf::from(format!("./SavedFile/{}", task.media_file_name));
        #[cfg(target_os = "windows")]
        let source_path = PathBuf::from(format!(".\\SavedFile\\{}", task.media_file_name));
        #[cfg(target_os = "linux")]
        let pre_process_folder = PathBuf::from(format!("./PreProcess/{}", uuid));
        #[cfg(target_os = "windows")]
        let pre_process_folder = PathBuf::from(format!(".\\PreProcess\\{}", uuid));
        #[cfg(target_os = "linux")]
        let post_process_folder = PathBuf::from(format!("./PostProcess/{}", uuid));
        #[cfg(target_os = "windows")]
        let post_process_folder = PathBuf::from(format!(".\\PostProcess\\{}", uuid));
        let destination_path = pre_process_folder.clone().join(&task.media_file_name);
        fs::create_dir(&pre_process_folder).await
            .map_err(|err|
                error_entry!(IOEntry::CreateDirectoryError(pre_process_folder.display(), err)))?;
        fs::create_dir(&post_process_folder).await
            .map_err(|err|
                error_entry!(IOEntry::CreateDirectoryError(post_process_folder.display(), err)))?;
        fs::rename(&source_path, &destination_path).await
            .map_err(|err|
                error_entry!(IOEntry::MoveFileError(source_path.display(), destination_path.display(), err)))?;
        TaskManager::update_unprocessed(&task.uuid, 1).await;
        Ok(())
    }

    async fn prepare_pre_processing(pre_process_folder: &PathBuf, post_process_folder: &PathBuf,
                                    source_path: &PathBuf, destination_path: &PathBuf) -> Result<(), LogEntry>
    {
        fs::create_dir(&pre_process_folder).await
            .map_err(|err|
                error_entry!(IOEntry::CreateDirectoryError(pre_process_folder.display(), err)))?;
        fs::create_dir(&post_process_folder).await
            .map_err(|err|
                error_entry!(IOEntry::CreateDirectoryError(post_process_folder.display(), err)))?;
        fs::rename(&source_path, &destination_path).await
            .map_err(|err|
                error_entry!(IOEntry::MoveFileError(source_path.display(), destination_path.display(), err)))?;
        Ok(())
    }

    async fn video_pre_process(task: &mut Task) -> Result<(), LogEntry> {
        let uuid = task.uuid.to_string();
        #[cfg(target_os = "linux")]
        let source_path = PathBuf::from(format!("./SavedFile/{}", task.media_file_name));
        #[cfg(target_os = "windows")]
        let source_path = PathBuf::from(format!(".\\SavedFile\\{}", task.media_file_name));
        #[cfg(target_os = "linux")]
        let pre_process_folder = PathBuf::from(format!("./PreProcess/{}", uuid));
        #[cfg(target_os = "windows")]
        let pre_process_folder = PathBuf::from(format!(".\\PreProcess\\{}", uuid));
        #[cfg(target_os = "linux")]
        let post_process_folder = PathBuf::from(format!("./PostProcess/{}", uuid));
        #[cfg(target_os = "windows")]
        let post_process_folder = PathBuf::from(format!(".\\PostProcess\\{}", uuid));
        let destination_path = pre_process_folder.clone().join(&task.media_file_name);
        Self::prepare_pre_processing(&pre_process_folder, &post_process_folder, &source_path, &destination_path).await?;
        let video_path = destination_path;
        Self::fetch_video_info(&video_path).await?;
        let cancel_flag = Self::instance().await.cancel_flag.clone();
        Self::split_video(video_path, cancel_flag).await?;
        let count = Self::file_count(&pre_process_folder).await?;
        TaskManager::update_unprocessed(&task.uuid, count).await;
        Ok(())
    }

    async fn fetch_video_info(video_path: &PathBuf) -> Result<(), LogEntry> {
        let absolute_path = video_path.canonicalize()
            .map_err(|err| error_entry!(IOEntry::GetAbsolutePathError(video_path.display(), err)))?;
        let absolute_path_str = absolute_path.to_string_lossy();
        let discoverer = Discoverer::new(gstreamer::ClockTime::from_seconds(5))
            .map_err(|err| error_entry!(GStreamerEntry::CreatePipelineError(err)))?;
        let info = discoverer.discover_uri(&*format!("file:///{absolute_path_str}"))
            .map_err(|err| error_entry!(GStreamerEntry::CreatePipelineError(err)))?;
        let mut video_info = VideoInfo::default();
        if let Some(stream) = info.video_streams().get(0) {
            video_info.bitrate = stream.bitrate();
            if let Some(caps) = stream.caps() {
                if let Some(structure) = caps.structure(0) {
                    video_info.format = structure.name().to_string();
                    for field in structure.fields() {
                        match field.as_str() {
                            "width" => video_info.width = structure.get::<i32>(field).unwrap_or_default(),
                            "height" => video_info.height = structure.get::<i32>(field).unwrap_or_default(),
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
            .map_err(|err| error_entry!(IOEntry::TomlSerializeError(err)))?;
        fs::write(&toml_path, video_info).await
            .map_err(|err| error_entry!(IOEntry::WriteFileError(toml_path.display(), err)))?;
        Ok(())
    }

    async fn split_video(video_path: PathBuf, cancel_flag: Arc<AtomicBool>) -> Result<(), LogEntry> {
        let config = Config::now().await;
        match config.split_mode {
            SplitMode::Frame => spawn_blocking(move || {
                Self::split_video_into_frames(video_path, cancel_flag)
            }).await,
            SplitMode::Time { segment_duration_secs } => spawn_blocking(move || {
                let segment_duration = Duration::from_secs(segment_duration_secs);
                Self::split_video_into_parts(video_path, segment_duration, cancel_flag)
            }).await
        }.map_err(|err| error_entry!(SystemEntry::TaskPanickedError(err)))??;
        Ok(())
    }

    fn split_video_into_frames(video_path: PathBuf, cancel_flag: Arc<AtomicBool>) -> Result<(), LogEntry> {
        let config = Config::now_blocking();
        let mut saved_path = video_path.clone();
        saved_path.pop();
        let pipeline_string = format!(
            "filesrc location={:?} ! decodebin ! videoconvert ! \
            video/x-raw,format=RGBA ! pngenc ! multifilesink location={:?}",
            video_path, saved_path.join("Frame_%d.png")
        );
        let pipeline = gstreamer::parse::launch(&pipeline_string)
            .map_err(|err| error_entry!(GStreamerEntry::CreatePipelineError(err)))?;
        let bus = pipeline.bus()
            .ok_or(error_entry!(GStreamerEntry::GetBusError))?;
        pipeline.set_state(gstreamer::State::Playing)
            .map_err(|err| error_entry!(GStreamerEntry::PipelineSetStateError(err)))?;
        let polling_interval = gstreamer::ClockTime::from_mseconds(config.polling_interval);
        let result = loop {
            if cancel_flag.load(Ordering::Relaxed) {
                break Err(information_entry!(SystemEntry::Cancel));
            }
            if let Some(message) = bus.timed_pop(polling_interval) {
                match message.view() {
                    gstreamer::MessageView::Eos(..) => break Ok(()),
                    gstreamer::MessageView::Error(err) => break Err(
                        error_entry!(GStreamerEntry::InternalError(err.error()))
                    ),
                    _ => {}
                }
            }
        };
        pipeline.set_state(gstreamer::State::Null)
            .map_err(|err| error_entry!(GStreamerEntry::PipelineSetStateError(err)))?;
        result
    }

    fn split_video_into_parts(video_path: PathBuf,
                              segment_duration: Duration, cancel_flag: Arc<AtomicBool>) -> Result<(), LogEntry>
    {
        let config = Config::now_blocking();
        let mut saved_path = video_path.clone();
        saved_path.pop();
        let duration_ns = segment_duration.as_nanos() as i64;
        let pipeline_string = format!(
            "filesrc location={:?} ! decodebin ! videoconvert ! x264enc ! \
            splitmuxsink location={:?} max-size-time={}",
            video_path, saved_path.join("Part_%d.mp4"), duration_ns
        );
        let pipeline = gstreamer::parse::launch(&pipeline_string)
            .map_err(|err| error_entry!(GStreamerEntry::CreatePipelineError(err)))?;
        let bus = pipeline.bus().ok_or(error_entry!(GStreamerEntry::GetBusError))?;
        pipeline.set_state(gstreamer::State::Playing)
            .map_err(|err| error_entry!(GStreamerEntry::PipelineSetStateError(err)))?;
        let polling_interval = gstreamer::ClockTime::from_mseconds(config.polling_interval);
        let result = loop {
            if cancel_flag.load(Ordering::Relaxed) {
                break Err(information_entry!(SystemEntry::Cancel));
            }
            if let Some(message) = bus.timed_pop(polling_interval) {
                match message.view() {
                    gstreamer::MessageView::Eos(..) => break Ok(()),
                    gstreamer::MessageView::Error(err) => break Err(
                        error_entry!(GStreamerEntry::InternalError(err.error()))
                    ),
                    _ => {}
                }
            }
        };
        pipeline.set_state(gstreamer::State::Null)
            .map_err(|err| error_entry!(GStreamerEntry::PipelineSetStateError(err)))?;
        result
    }

    async fn zip_pre_process(task: &mut Task) -> Result<(), LogEntry> {
        let uuid = task.uuid.to_string();
        #[cfg(target_os = "linux")]
        let source_path = PathBuf::from(format!("./SavedFile/{}", task.media_file_name));
        #[cfg(target_os = "windows")]
        let source_path = PathBuf::from(format!(".\\SavedFile\\{}", task.media_file_name));
        #[cfg(target_os = "linux")]
        let pre_process_folder = PathBuf::from(format!("./PreProcess/{}", uuid));
        #[cfg(target_os = "windows")]
        let pre_process_folder = PathBuf::from(format!(".\\PreProcess\\{}", uuid));
        #[cfg(target_os = "linux")]
        let post_process_folder = PathBuf::from(format!("./PostProcess/{}", uuid));
        #[cfg(target_os = "windows")]
        let post_process_folder = PathBuf::from(format!(".\\PostProcess\\{}", uuid));
        let destination_path = pre_process_folder.clone().join(&task.media_file_name);
        Self::prepare_pre_processing(&pre_process_folder, &post_process_folder, &source_path, &destination_path).await?;
        let zip_path = destination_path;
        let cancel_flag = Self::instance().await.cancel_flag.clone();
        Self::unzip(zip_path, cancel_flag).await?;
        let count = Self::file_count(&pre_process_folder).await?;
        TaskManager::update_unprocessed(&task.uuid, count).await;
        Ok(())
    }

    async fn unzip(zip_path: PathBuf, cancel_flag: Arc<AtomicBool>) -> Result<(), LogEntry> {
        spawn_blocking(move || {
            Self::unzip_blocking(zip_path, cancel_flag)
        }).await
            .map_err(|err| error_entry!(SystemEntry::TaskPanickedError(err)))??;
        Ok(())
    }

    fn unzip_blocking(zip_path: PathBuf, cancel_flag: Arc<AtomicBool>) -> Result<(), LogEntry> {
        let allowed_extensions = ["png", "jpg", "jpeg"];
        let reader = File::open(&zip_path)
            .map_err(|err| error_entry!(IOEntry::ReadFileError(zip_path.display(), err)))?;
        let mut archive = ZipArchive::new(reader)
            .map_err(|err|
                error_entry!(IOEntry::ReadFileError(zip_path.display(), std::io::Error::from(err))))?;
        let mut saved_path = zip_path.clone();
        saved_path.pop();
        for i in 0..archive.len() {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(information_entry!("Operation cancelled"));
            }
            let mut file = archive.by_index(i)
                .map_err(|err|
                    error_entry!(IOEntry::ReadFileError(zip_path.display(), std::io::Error::from(err))))?;
            if let Some(enclosed_path) = file.enclosed_name() {
                if let Some(extension) = enclosed_path.extension() {
                    if allowed_extensions.contains(&extension.to_str().unwrap_or("")) {
                        let output_path = saved_path.join(enclosed_path.file_name().unwrap_or_default());
                        let mut output_file = File::create(&output_path)
                            .map_err(|err| error_entry!(IOEntry::CreateFileError(output_path.display(), err)))?;
                        std::io::copy(&mut file, &mut output_file)
                            .map_err(|err| error_entry!(IOEntry::WriteFileError(output_path.display(), err)))?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn picture_post_processing(task: &mut Task) -> Result<(), LogEntry> {
        let inference_task = task.result.get(0)
            .ok_or(error_entry!("Missing tasks"))?;
        #[cfg(target_os = "linux")]
        let source_path = PathBuf::from(format!("./PostProcess/{}", inference_task.media_file_name));
        #[cfg(target_os = "windows")]
        let source_path = PathBuf::from(format!(".\\PostProcess\\{}", inference_task.media_file_name));
        #[cfg(target_os = "linux")]
        let destination_path = PathBuf::from(format!("./Result/{}", inference_task.media_file_name));
        #[cfg(target_os = "windows")]
        let destination_path = PathBuf::from(format!(".\\Result\\{}", inference_task.media_file_name));
        fs::rename(&source_path, &destination_path).await
            .map_err(|err|
                error_entry!(IOEntry::MoveFileError(source_path.display(), destination_path.display(), err)))?;
        Ok(())
    }

    async fn prepare_post_processing(media_file_name: String,
                                     pre_process: &PathBuf, post_process: &PathBuf) -> Result<(), LogEntry>
    {
        let ignore_file = pre_process.join(&media_file_name);
        let mut dir = fs::read_dir(pre_process).await
            .map_err(|err| error_entry!(IOEntry::ReadDirectoryError(pre_process.display(), err)))?;
        while let Some(entry) = dir.next_entry().await
            .map_err(|err| error_entry!(IOEntry::ReadDirectoryError(pre_process.display(), err)))?
        {
            let path = entry.path();
            if path == ignore_file {
                continue;
            }
            if path.is_file() {
                let file_name = match path.file_name() {
                    Some(name) => name,
                    None => continue,
                };
                let destination_path = post_process.join(file_name);
                if destination_path.exists() {
                    continue;
                }
                fs::rename(&path, &destination_path).await
                    .map_err(|err|
                        error_entry!(IOEntry::MoveFileError(path.display(), destination_path.display(), err)))?;
            }
        }
        Ok(())
    }

    async fn video_post_processing(task: &mut Task) -> Result<(), LogEntry> {
        let uuid = task.uuid.to_string();
        let media_file_name = task.media_file_name.clone();
        #[cfg(target_os = "linux")]
        let pre_process_folder = PathBuf::from(format!("./PreProcess/{}", uuid));
        #[cfg(target_os = "windows")]
        let pre_process_folder = PathBuf::from(format!(".\\PreProcess\\{}", uuid));
        #[cfg(target_os = "linux")]
        let post_process_folder = PathBuf::from(format!("./PostProcess/{}", uuid));
        #[cfg(target_os = "windows")]
        let post_process_folder = PathBuf::from(format!(".\\PostProcess\\{}", uuid));
        let video_info_path = post_process_folder.clone().join(&task.media_file_name).with_extension("toml");
        let saved_path = post_process_folder.clone().join(&task.media_file_name);
        Self::prepare_post_processing(media_file_name, &pre_process_folder, &post_process_folder).await?;
        let cancel_flag = Self::instance().await.cancel_flag.clone();
        Self::recombination_video(video_info_path, post_process_folder, saved_path, cancel_flag).await?;
        Self::move_result(&task).await?;
        Ok(())
    }

    async fn recombination_video(video_info_path: PathBuf, post_process_folder: PathBuf,
                                 saved_path: PathBuf, cancel_flag: Arc<AtomicBool>) -> Result<(), LogEntry>
    {
        let config = Config::now().await;
        match config.split_mode {
            SplitMode::Frame => spawn_blocking(move || {
                Self::recombination_video_from_frame(video_info_path, post_process_folder, saved_path, cancel_flag)
            }).await,
            SplitMode::Time { .. } => spawn_blocking(move || {
                Self::recombination_video_from_partial(post_process_folder, saved_path, cancel_flag)
            }).await
        }.map_err(|err| error_entry!(SystemEntry::TaskPanickedError(err)))??;
        Ok(())
    }

    fn recombination_video_from_frame(video_info_path: PathBuf, frame_folder: PathBuf,
                                      saved_path: PathBuf, cancel_flag: Arc<AtomicBool>) -> Result<(), LogEntry>
    {
        let config = Config::now_blocking();
        let toml_str = std::fs::read_to_string(&video_info_path)
            .map_err(|err| error_entry!(IOEntry::ReadFileError(video_info_path.display(), err)))?;
        let video_info: VideoInfo = toml::from_str(&toml_str).unwrap_or_default();
        let bitrate = video_info.bitrate;
        let encoder = format!("x264enc bitrate={}", bitrate / 1000);
        let muxer = "mp4mux";
        let pipeline_string = format!(
            "multifilesrc location={:?} index=1 caps=image/png, framerate=(fraction){} ! \
            pngdec ! videoconvert ! {} ! {} ! filesink location={:?}",
            frame_folder.join("Frame_%d.png"), video_info.framerate, encoder, muxer, saved_path
        );
        let pipeline = gstreamer::parse::launch(&pipeline_string)
            .map_err(|err| error_entry!(GStreamerEntry::CreatePipelineError(err)))?;
        let bus = pipeline.bus().ok_or(error_entry!(GStreamerEntry::GetBusError))?;
        pipeline.set_state(gstreamer::State::Playing)
            .map_err(|err| error_entry!(GStreamerEntry::PipelineSetStateError(err)))?;
        let polling_interval = gstreamer::ClockTime::from_mseconds(config.polling_interval);
        let result = loop {
            if cancel_flag.load(Ordering::Relaxed) {
                break Err(information_entry!(SystemEntry::Cancel));
            }
            if let Some(message) = bus.timed_pop(polling_interval) {
                match message.view() {
                    gstreamer::MessageView::Eos(..) => break Ok(()),
                    gstreamer::MessageView::Error(err) => break Err(
                        error_entry!(GStreamerEntry::InternalError(err.error()))
                    ),
                    _ => {}
                }
            }
        };
        pipeline.set_state(gstreamer::State::Null)
            .map_err(|err| error_entry!(GStreamerEntry::PipelineSetStateError(err)))?;
        result
    }

    fn recombination_video_from_partial(partial_video_folder: PathBuf, saved_path: PathBuf,
                                        cancel_flag: Arc<AtomicBool>) -> Result<(), LogEntry>
    {
        let config = Config::now_blocking();
        let mut part_files: Vec<PathBuf> = std::fs::read_dir(&partial_video_folder)
            .map_err(|err|
                error_entry!(IOEntry::ReadDirectoryError(partial_video_folder.display(), err)))?
            .filter_map(|entry| {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                            if file_name.starts_with("Part_") && file_name.ends_with(".mp4") {
                                return Some(path);
                            }
                        }
                    }
                }
                None
            })
            .collect();
        part_files.sort_by_key(|path| {
            path.file_name()
                .and_then(|os_str| os_str.to_str())
                .and_then(|str| {
                    str.trim_start_matches("Part_")
                        .trim_end_matches(".mp4")
                        .parse::<u32>()
                        .ok()
                })
        });
        let input_sources: String = part_files.iter().enumerate()
            .map(|(index, path)| {
                let path_str = path.to_string_lossy().replace("\\", "/");
                format!(
                    "filesrc location=\"{}\" ! \
                    qtdemux name=demux{index} \
                    demux{index}.video_0 ! queue ! h264parse ! video_concat. ",
                    path_str,
                    index = index
                )
            })
            .collect::<Vec<String>>()
            .join(" ");
        #[cfg(target_os = "linux")]
        let saved_path_str = saved_path.to_string_lossy();
        #[cfg(target_os = "windows")]
        let saved_path_str = saved_path.to_string_lossy().replace("\\", "/");
        let pipeline_string = format!(
            "{} concat name=video_concat ! queue ! mux.video_0 \
            qtmux name=mux ! filesink location={}",
            input_sources,
            saved_path_str,
        );
        let pipeline = gstreamer::parse::launch(&pipeline_string)
            .map_err(|err| error_entry!(GStreamerEntry::CreatePipelineError(err)))?;
        let bus = pipeline.bus().ok_or(error_entry!(GStreamerEntry::GetBusError))?;
        pipeline.set_state(gstreamer::State::Playing)
            .map_err(|err| error_entry!(GStreamerEntry::PipelineSetStateError(err)))?;
        let polling_interval = gstreamer::ClockTime::from_mseconds(config.polling_interval);
        let result = loop {
            if cancel_flag.load(Ordering::Relaxed) {
                break Err(information_entry!(SystemEntry::Cancel));
            }
            if let Some(message) = bus.timed_pop(polling_interval) {
                match message.view() {
                    gstreamer::MessageView::Eos(..) => break Ok(()),
                    gstreamer::MessageView::Error(err) => break Err(
                        error_entry!(GStreamerEntry::InternalError(err.error()))
                    ),
                    _ => {}
                }
            }
        };
        pipeline.set_state(gstreamer::State::Null)
            .map_err(|err| error_entry!("Unable to set pipeline state", format!("Err: {err}")))?;
        result
    }

    async fn zip_post_processing(task: &mut Task) -> Result<(), LogEntry> {
        let uuid = task.uuid.to_string();
        let media_file_name = task.media_file_name.clone();
        #[cfg(target_os = "linux")]
        let pre_process_folder = PathBuf::from(format!("./PreProcess/{}", uuid));
        #[cfg(target_os = "windows")]
        let pre_process_folder = PathBuf::from(format!(".\\PreProcess\\{}", uuid));
        #[cfg(target_os = "linux")]
        let post_process_folder = PathBuf::from(format!("./PostProcess/{}", uuid));
        #[cfg(target_os = "windows")]
        let post_process_folder = PathBuf::from(format!(".\\PostProcess\\{}", uuid));
        let saved_path = post_process_folder.clone().join(&task.media_file_name);
        Self::prepare_post_processing(media_file_name, &pre_process_folder, &post_process_folder).await?;
        let cancel_flag = Self::instance().await.cancel_flag.clone();
        Self::recombination_zip(pre_process_folder, saved_path, cancel_flag).await?;
        Self::move_result(&task).await?;
        Ok(())
    }

    async fn recombination_zip(pre_process_folder: PathBuf,
                               target_path: PathBuf, cancel_flag: Arc<AtomicBool>) -> Result<(), LogEntry>
    {
        spawn_blocking(move || {
            Self::recombination_zip_blocking(pre_process_folder, target_path, cancel_flag)
        }).await
            .map_err(|err| error_entry!(SystemEntry::TaskPanickedError(err)))??;
        Ok(())
    }

    fn recombination_zip_blocking(source_folder: PathBuf,
                                  target_path: PathBuf, cancel_flag: Arc<AtomicBool>) -> Result<(), LogEntry>
    {
        let file = File::create(&target_path)
            .map_err(|err| error_entry!(IOEntry::CreateFileError(target_path.display(), err)))?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for entry in std::fs::read_dir(&source_folder)
            .map_err(|err| error_entry!(IOEntry::ReadDirectoryError(source_folder.display(), err)))?
        {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(information_entry!(SystemEntry::Cancel));
            }
            let entry = entry
                .map_err(|err| error_entry!(IOEntry::ReadDirectoryError(source_folder.display(), err)))?;
            let path = entry.path();
            let file_name = path.file_name()
                .ok_or(error_entry!(MiscEntry::InvalidFileNameError))?.to_string_lossy();
            zip.start_file(&file_name, options)
                .map_err(|err|
                    error_entry!(IOEntry::CreateFileError(path.display(), std::io::Error::from(err))))?;
            let mut file_contents = Vec::new();
            File::open(&path)
                .map_err(|err| error_entry!(IOEntry::ReadFileError(path.display(), err)))?
                .read_to_end(&mut file_contents)
                .map_err(|err| error_entry!(IOEntry::ReadFileError(path.display(), err)))?;
            zip.write_all(&file_contents)
                .map_err(|err| error_entry!(IOEntry::WriteFileError(path.display(), err)))?;
        }
        zip.finish()
            .map_err(|err|
                error_entry!(IOEntry::WriteFileError(target_path.display(), std::io::Error::from(err))))?;
        Ok(())
    }

    async fn move_result(task: &Task) -> Result<(), LogEntry> {
        let uuid = task.uuid.to_string();
        #[cfg(target_os = "linux")]
        let media_file_path = PathBuf::from(format!("./PostProcess/{}/{}", uuid, task.media_file_name));
        #[cfg(target_os = "windows")]
        let media_file_path = PathBuf::from(format!(".\\PostProcess\\{}\\{}", uuid, task.media_file_name));
        #[cfg(target_os = "linux")]
        let destination_path = PathBuf::from(format!("./Result/{}", task.media_file_name));
        #[cfg(target_os = "windows")]
        let destination_path = PathBuf::from(format!(".\\Result\\{}", task.media_file_name));
        fs::rename(&media_file_path, &destination_path).await
            .map_err(|err|
                error_entry!(IOEntry::MoveFileError(media_file_path.display(), destination_path.display(), err)))?;
        Ok(())
    }

    async fn file_count(path: &PathBuf) -> Result<usize, LogEntry> {
        let read_dir = fs::read_dir(path).await
            .map_err(|err| error_entry!(IOEntry::ReadDirectoryError(path.display(), err)))?;
        let dir_entries = ReadDirStream::new(read_dir);
        let count = dir_entries.filter_map(|entry| async {
            entry.ok().and_then(|e| if e.path().is_file() { Some(()) } else { None })
        }).count().await;
        Ok(count - 2)
    }
}
