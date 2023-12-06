use tokio::fs;
use uuid::Uuid;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::manager::node::Node;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::file_manager::FileManager;
use crate::manager::node_cluster::NodeCluster;
use crate::manager::utils::image_task::ImageTask;
use crate::manager::utils::task::{Task, TaskStatus};
use crate::manager::result_repository::ResultRepository;

lazy_static! {
    static ref GLOBAL_TASK_MANAGER: RwLock<TaskManager> = RwLock::new(TaskManager::new());
}

pub struct TaskManager {
    tasks: HashMap<Uuid, Task>,
}

impl TaskManager {
    fn new() -> TaskManager {
        TaskManager {
            tasks: HashMap::new(),
        }
    }

    pub async fn instance() -> RwLockReadGuard<'static, TaskManager> {
        GLOBAL_TASK_MANAGER.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, TaskManager> {
        GLOBAL_TASK_MANAGER.write().await
    }

    pub async fn run() {

    }

    pub async fn add_task(mut task: Task) {
        let mut task_manager = TaskManager::instance_mut().await;
        task.status = TaskStatus::Processing;
        task_manager.tasks.insert(task.uuid, task.clone());
        tokio::spawn(async move {
            TaskManager::distribute_task(task).await;
        });
    }

    pub async fn remove_task(uuid: &Uuid) -> Option<Task> {
        let mut task_manager = TaskManager::instance_mut().await;
        task_manager.tasks.remove(&uuid)
    }

    pub async fn get_task(&self, uuid: &Uuid) -> Option<&Task> {
        self.tasks.get(&uuid)
    }

    pub async fn get_task_mut(&mut self, uuid: &Uuid) -> Option<&mut Task> {
        self.tasks.get_mut(&uuid)
    }

    pub async fn distribute_task(mut task: Task) {
        let model_filepath = Path::new(".").join("SavedModel").join(task.model_filename.clone());
        let nodes = NodeCluster::sorted_by_vram().await;
        let vram_usage = TaskManager::calc_vram_usage(&model_filepath).await;
        let filter_nodes = NodeCluster::filter_node_by_vram(vram_usage).await;
        match Path::new(&task.media_filename).extension().and_then(|os_str| os_str.to_str()) {
            Some("png") | Some("jpg") | Some("jpeg") => {
                let mut node: Option<usize> = None;
                let image_filepath = Path::new(".").join("PreProcessing").join(task.media_filename.clone());
                let mut image_resource = ImageTask::new(task.uuid, model_filepath, image_filepath.clone(), task.inference_type);
                let ram_usage = TaskManager::calc_ram_usage(image_filepath).await;
                for (node_id, _) in filter_nodes {
                    let node_ram = match NodeCluster::get_node(node_id).await {
                        Some(node) => node.read().await.idle_unused().ram,
                        None => {
                            Logger::append_system_log(LogLevel::WARNING, format!("Task Manager: Node {} does not exist.", node_id)).await;
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
                        match NodeCluster::get_node(node_id).await {
                            Some(node) => Node::add_task(node, image_resource).await,
                            None => {
                                TaskManager::handle_image_task(&task.uuid, false).await;
                                Logger::append_system_log(LogLevel::WARNING, format!("Task Manager: Node {} does not exist.", node_id)).await;
                                Logger::append_system_log(LogLevel::WARNING, format!("Task Manager: Task {} cannot be assigned to any node.", task.uuid)).await;
                            }
                        }
                    },
                    None => {
                        TaskManager::handle_image_task(&task.uuid, false).await;
                        Logger::append_system_log(LogLevel::WARNING, format!("Task Manager: Task {} cannot be assigned to any node.", task.uuid)).await;
                    }
                }
            },
            Some("gif") | Some("mp4") | Some("wav") | Some("avi") | Some("mkv") | Some("zip") => {
                let inference_folder = Path::new(".").join("PreProcessing").join(task.media_filename.clone()).with_extension("");
                let mut inference_folder = match fs::read_dir(&inference_folder).await {
                    Ok(inference_folder) => inference_folder,
                    Err(_) => {
                        Logger::append_system_log(LogLevel::ERROR, format!("Task Manager: Cannot read folder {}.", inference_folder.display())).await;
                        return;
                    },
                };
                let mut current_node: usize = 0;
                while let Ok(Some(image_filepath)) = inference_folder.next_entry().await {
                    let image_filepath = image_filepath.path();
                    let mut image_resource = ImageTask::new(task.uuid, model_filepath.clone(), image_filepath.clone(), task.inference_type);
                    let ram_usage = TaskManager::calc_ram_usage(image_filepath).await;
                    let mut node: Option<usize> = None;
                    for i in 0..nodes.len() {
                        let index = (current_node + i) % filter_nodes.len();
                        let node_id = match nodes.get(index) {
                            Some((node_id, _)) => *node_id,
                            None => continue,
                        };
                        let node_ram = match NodeCluster::get_node(node_id).await {
                            Some(node) => node.read().await.idle_unused().ram,
                            None => {
                                Logger::append_system_log(LogLevel::WARNING, format!("Task Manager: Node {} does not exist.", node_id)).await;
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
                            match NodeCluster::get_node(node_id).await {
                                Some(node) => Node::add_task(node, image_resource).await,
                                None => {
                                    TaskManager::handle_image_task(&task.uuid, false).await;
                                    Logger::append_system_log(LogLevel::WARNING, format!("Task Manager: Node {} does not exist.", node_id)).await;
                                }
                            }
                        },
                        None => TaskManager::handle_image_task(&task.uuid, false).await,
                    }
                }
            },
            _ => {
                let error_message = format!("Task Manager: Task {} failed because the file extension is not supported.", task.uuid);
                TaskManager::task_panic(&task.uuid, error_message.clone()).await;
                Logger::append_system_log(LogLevel::INFO, error_message).await;
            },
        }
    }

    pub async fn task_panic(uuid: &Uuid, error: String) {
        let mut task_manager = TaskManager::instance_mut().await;
        if let Some(mut task) = task_manager.tasks.remove(uuid) {
            task.panic(error);
            task.change_status(TaskStatus::Fail);
            ResultRepository::add_task(task).await;
        }
    }

    pub async fn handle_image_task(uuid: &Uuid, success: bool) {
        let mut complete = false;
        let mut task_manager = TaskManager::instance_mut().await;
        match task_manager.tasks.get_mut(&uuid) {
            Some(task) => {
                task.unprocessed -= 1;
                if success {
                    task.success += 1;
                } else {
                    task.failed += 1;
                }
                if task.unprocessed == 0 {
                    complete = true;
                }
            },
            None => Logger::append_system_log(LogLevel::ERROR, format!("Task Manager: Task {} does not exist.", uuid)).await,
        }
        if complete {
            if let Some(task) = task_manager.tasks.remove(&uuid) {
                FileManager::add_postprocess_task(task).await;
            }
        }
    }

    async fn calc_vram_usage(model_filepath: &PathBuf) -> f64 {
        let model_filesize = match fs::metadata(model_filepath).await {
            Ok(metadata) => metadata.len(),
            Err(_) => {
                Logger::append_system_log(LogLevel::ERROR, format!("Task Manager: Cannot read file {}.", model_filepath.display())).await;
                0
            }
        };
        2.4319e-6 * model_filesize as f64 + 303.3889
    }

    async fn calc_ram_usage(image_filepath: &PathBuf) -> f64 {
        let image_filesize = match fs::metadata(image_filepath).await {
            Ok(metadata) => metadata.len(),
            Err(_) => {
                Logger::append_system_log(LogLevel::ERROR, format!("Task Manager: Cannot read file {}.", image_filepath.display())).await;
                0
            }
        };
        4.1894 * image_filesize as f64 + 1_398_237_298.688
    }
}
