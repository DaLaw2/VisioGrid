use tokio::fs;
use uuid::Uuid;
use tokio::sync::Mutex;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::node_cluster::NodeCluster;
use crate::manager::utils::task::{Task, TaskStatus};
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

    pub async fn add_task(mut task: Task) {
        let mut task_manager = GLOBAL_TASK_MANAGER.lock().await;
        task.status = TaskStatus::Processing;
        task_manager.tasks.insert(task.uuid, task.clone());
        tokio::spawn(async move {
            TaskManager::distribute_task(task).await;
        });
    }

    pub async fn remove_task(uuid: Uuid) -> Option<Task> {
        let mut task_manager = GLOBAL_TASK_MANAGER.lock().await;
        task_manager.tasks.remove(&uuid)
    }

    pub async fn distribute_task(mut task: Task) {
        let model_filepath = Path::new(".").join("SavedModel").join(task.model_filename.clone());
        let vram_usage = Self::calc_vram_usage(model_filepath.clone()).await;
        let nodes = NodeCluster::sort_by_vram().await;
        let filter_nodes = NodeCluster::filter_node_by_vram(vram_usage).await;
        match Path::new(&task.image_filename).extension().and_then(|os_str| os_str.to_str()) {
            Some("png") | Some("jpg") | Some("jpeg") => {
                let image_filepath = Path::new(".").join("PreProcessing").join(task.image_filename.clone());
                let mut image_resource = ImageResource::new(task.uuid, model_filepath.clone(), image_filepath.clone(), task.inference_type.clone());
                let ram_usage = Self::calc_ram_usage(image_filepath).await;
                let mut node: Option<usize> = None;
                for (node_id, _) in filter_nodes {
                    let node_ram = match NodeCluster::instance().await.get_node(node_id) {
                        Some(node) => node.idle_unused.ram,
                        None => {
                            Logger::append_global_log(LogLevel::WARNING, format!("Task Manager: Node {} does not exist.", node_id)).await;
                            0.0
                        }
                    };
                    if node_ram > ram_usage * 0.7 {
                        node = Some(node_id);
                        if node_ram < ram_usage {
                            image_resource.cache = true;
                        }
                        break;
                    }
                }
                match node {
                    Some(node_id) => {
                        match NodeCluster::instance_mut().await.get_node_mut(node_id) {
                            Some(node) => node.add_task(image_resource).await,
                            None => {
                                Logger::append_global_log(LogLevel::WARNING, format!("Task Manager: Node {} does not exist.", node_id)).await;
                                Logger::append_global_log(LogLevel::ERROR, format!("Task Manager: Task {} cannot be assigned to any node.", task.uuid)).await;
                                task.status = TaskStatus::Fail;
                                let _ = Self::remove_task(task.uuid).await;
                            }
                        }
                    }
                    None => {
                        Logger::append_global_log(LogLevel::ERROR, format!("Task Manager: Task {} cannot be assigned to any node.", task.uuid)).await;
                        task.status = TaskStatus::Fail;
                        let _ = Self::remove_task(task.uuid).await;
                    }
                }
            },
            Some("gif") | Some("mp4") | Some("wav") | Some("avi") | Some("mkv") | Some("zip") => {
                let inference_folder = Path::new(".").join("PreProcessing").join(task.image_filename.clone()).with_extension("");
                let mut inference_folder = match fs::read_dir(&inference_folder).await {
                    Ok(inference_folder) => inference_folder,
                    Err(_) => {
                        Logger::append_global_log(LogLevel::ERROR, format!("Task Manager: Cannot read folder {}.", inference_folder.display())).await;
                        return;
                    },
                };
                let mut current_node = 0_usize;
                while let Ok(Some(image_filepath)) = inference_folder.next_entry().await {
                    let image_filepath = image_filepath.path();
                    let mut image_resource = ImageResource::new(task.uuid, model_filepath.clone(), image_filepath.clone(), task.inference_type.clone());
                    let ram_usage = Self::calc_ram_usage(image_filepath).await;
                    let mut node: Option<usize> = None;
                    for i in 0..nodes.len() {
                        let index = (current_node + i) % filter_nodes.len();
                        let node_id = match nodes.get(index) {
                            Some((node_id, _)) => *node_id,
                            None => continue,
                        };
                        let node_ram = match NodeCluster::instance().await.get_node(node_id) {
                            Some(node) => node.idle_unused.ram,
                            None => {
                                Logger::append_global_log(LogLevel::WARNING, format!("Task Manager: Node {} does not exist.", node_id)).await;
                                0.0
                            }
                        };
                        if node_ram > ram_usage * 0.7 {
                            node = Some(node_id);
                            current_node += 1;
                            if node_ram < ram_usage {
                                image_resource.cache = true;
                            }
                            break;
                        }
                    }
                    match node {
                        Some(node_id) => {
                            match NodeCluster::instance_mut().await.get_node_mut(node_id) {
                                Some(node) => node.add_task(image_resource).await,
                                None => Logger::append_global_log(LogLevel::WARNING, format!("Task Manager: Node {} does not exist.", node_id)).await
                            }
                        },
                        None => {}
                    }
                }
                if task.processed == task.unprocessed {
                    let _ = Self::remove_task(task.uuid).await;
                    task.status = TaskStatus::Fail;
                    return;
                }
            },
            _ => {
                Logger::append_global_log(LogLevel::ERROR, format!("Task Manager: Task {} failed because the file extension is not supported.", task.uuid)).await;
                task.status = TaskStatus::Fail;
                let _ = Self::remove_task(task.uuid).await;
            },
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
