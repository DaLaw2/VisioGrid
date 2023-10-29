use tokio::fs;
use tokio::sync::Mutex;
use lazy_static::lazy_static;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use crate::manager::node_cluster::NodeCluster;
use crate::manager::utils::task::Task;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::utils::image_resource::ImageResource;

lazy_static! {
    static ref GLOBAL_TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}

pub struct TaskManager {
    tasks: HashMap<Uuid, Task>,
}

impl TaskManager {
    fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    pub async fn run() {

    }

    pub async fn add_task(task: Task) {
        let mut task_manager = GLOBAL_TASK_MANAGER.lock().await;
        task_manager.tasks.insert(task.uuid, task.clone());
        // TaskManager::distribute_task(task).await;
    }

    pub async fn distribute_task(task: Task) {
        let model_filepath = Path::new(".").join("SavedModel").join(task.model_filename.clone());
        let vram_usage = Self::calc_vram_usage(model_filepath.clone()).await;
        let node_vram_sorting = NodeCluster::get_vram_sorting().await;
        match Path::new(&task.image_filename).extension().and_then(|os_str| os_str.to_str()) {
            Some("png") | Some("jpg") | Some("jpeg") => {
                let image_filepath = Path::new(".").join("PreProcessing").join(task.image_filename.clone());
                let image_resource = ImageResource::new(task.uuid, model_filepath.clone(), image_filepath, task.inference_type.clone());
            },
            Some("gif") | Some("mp4") | Some("wav") | Some("avi") | Some("mkv") | Some("zip") => {
                let inference_folder = Path::new(".").join("PreProcessing").join(task.image_filename.clone()).with_extension("");
                let mut inference_folder = match fs::read_dir(&inference_folder).await {
                    Ok(inference_folder) => inference_folder,
                    Err(err) => {
                        Logger::append_global_log(LogLevel::ERROR, format!("Fail read folder {}: {:?}", inference_folder.display(), err)).await;
                        return;
                    },
                };
                while let Ok(Some(image_filepath)) = inference_folder.next_entry().await {
                    let image_filepath = image_filepath.path();
                    let image_resource = ImageResource::new(task.uuid, model_filepath.clone(), image_filepath, task.inference_type.clone());
                    ()
                }
            },
            _ => Logger::append_global_log(LogLevel::ERROR, "Add image to task manager failed.".to_string()).await,
        }
    }

    async fn calc_vram_usage(model_filepath: PathBuf) -> f64 {
        let model_filesize = match fs::metadata(&model_filepath).await {
            Ok(metadata) => metadata.len(),
            Err(_) => {
                Logger::append_global_log(LogLevel::ERROR, format!("Task Manager: Cannot read file {}.", model_filepath.display())).await;
                0
            }
        };
        2.4319e-6 * model_filesize as f64 + 303.3889
    }

    async fn calc_ram_usage(image_filepath: PathBuf) -> f64 {
        let image_filesize = match fs::metadata(&image_filepath).await {
            Ok(metadata) => metadata.len(),
            Err(_) => {
                Logger::append_global_log(LogLevel::ERROR, format!("Task Manager: Cannot read file {}.", image_filepath.display())).await;
                0
            }
        };
        4.1894 * image_filesize as f64 + 1_398_237_298.688
    }
}
