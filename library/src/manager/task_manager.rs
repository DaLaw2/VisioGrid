use tokio::fs;
use uuid::Uuid;
use lazy_static::lazy_static;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, VecDeque};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::manager::node::Node;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::file_manager::FileManager;
use crate::manager::node_cluster::NodeCluster;
use crate::manager::utils::image_task::ImageTask;
use crate::manager::utils::task::{Task, TaskStatus};

lazy_static! {
    static ref GLOBAL_TASK_MANAGER: RwLock<TaskManager> = RwLock::new(TaskManager::new());
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

    pub async fn instance() -> RwLockReadGuard<'static, Self> {
        GLOBAL_TASK_MANAGER.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        GLOBAL_TASK_MANAGER.write().await
    }

    pub async fn add_task(mut task: Task) {
        task.change_status(TaskStatus::Processing);
        {
            let mut task_manager = Self::instance_mut().await;
            task_manager.tasks.insert(task.uuid, task.clone());
        }
        tokio::spawn(async move {
            Self::distribute_task(task).await;
        });
    }

    pub async fn remove_task(uuid: &Uuid) -> Option<Task> {
        let mut task_manager = Self::instance_mut().await;
        task_manager.tasks.remove(&uuid)
    }

    pub async fn get_task(&self, uuid: &Uuid) -> Option<&Task> {
        self.tasks.get(&uuid)
    }

    pub async fn get_task_mut(&mut self, uuid: &Uuid) -> Option<&mut Task> {
        self.tasks.get_mut(&uuid)
    }

    pub async fn distribute_task(task: Task) {
        let model_filepath = Path::new(".").join("SavedModel").join(task.model_filename.clone());
        let vram_usage = Self::estimated_vram_usage(&model_filepath).await;
        let filter_nodes = NodeCluster::filter_node_by_vram(vram_usage).await;
        match Path::new(&task.media_filename).extension().and_then(|os_str| os_str.to_str()) {
            Some("png") | Some("jpg") | Some("jpeg") => {
                let image_filepath = Path::new(".").join("PreProcessing").join(task.media_filename.clone());
                let mut image_task = ImageTask::new(0_usize, &task, model_filepath, image_filepath.clone());
                let ram_usage = Self::estimated_ram_usage(&image_filepath).await;
                let mut assigned = false;
                for (node_id, _) in filter_nodes {
                    let node_ram = match NodeCluster::get_node(node_id).await {
                        Some(node) => node.read().await.idle_unused().ram,
                        None => {
                            Logger::append_system_log(LogLevel::WARNING, format!("Task Manager: Node {} does not exist.", node_id)).await;
                            0.0
                        }
                    };
                    if node_ram > ram_usage * 0.7 {
                        if let Some(node) = NodeCluster::get_node(node_id).await {
                            if node_ram < ram_usage {
                                image_task.cache = true;
                            }
                            Node::add_task(node, image_task.clone()).await;
                            assigned = true;
                            break;
                        }
                    }
                }
                if !assigned {
                    Logger::append_system_log(LogLevel::WARNING, format!("Task Manager: Task {} cannot be assigned to any node.", image_task.task_uuid)).await;
                    Self::submit_image_task(image_task, false).await;
                }
            }
            Some("mp4") | Some("wav") | Some("avi") | Some("mkv") | Some("zip") => {
                let image_folder = Path::new(".").join("PreProcessing").join(task.media_filename.clone()).with_extension("");
                let mut image_folder = match fs::read_dir(&image_folder).await {
                    Ok(image_folder) => image_folder,
                    Err(_) => {
                        Logger::append_system_log(LogLevel::ERROR, format!("Task Manager: Cannot read folder {}.", image_folder.display())).await;
                        return;
                    }
                };
                let mut image_id = 0_usize;
                let mut current_node = 0_usize;
                while let Ok(Some(image_filepath)) = image_folder.next_entry().await {
                    let image_filepath = image_filepath.path();
                    let mut image_task = ImageTask::new(image_id, &task, model_filepath.clone(), image_filepath.clone());
                    let ram_usage = Self::estimated_ram_usage(&image_filepath).await;
                    let mut assigned = false;
                    for i in 0..filter_nodes.len() {
                        let index = (current_node + i) % filter_nodes.len();
                        let node_id = match filter_nodes.get(index) {
                            Some((node_id, _)) => *node_id,
                            None => continue,
                        };
                        let node_ram = match NodeCluster::get_node(node_id).await {
                            Some(node) => node.read().await.idle_unused().ram,
                            None => {
                                Logger::append_system_log(LogLevel::ERROR, format!("Task Manager: Node {} does not exist.", node_id)).await;
                                0.0
                            }
                        };
                        if node_ram > ram_usage * 0.7 {
                            if let Some(node) = NodeCluster::get_node(node_id).await {
                                if node_ram < ram_usage {
                                    image_task.cache = true;
                                }
                                Node::add_task(node, image_task.clone()).await;
                                assigned = true;
                                current_node += 1;
                                break;
                            }
                        }
                    }
                    if !assigned {
                        Logger::append_system_log(LogLevel::WARNING, format!("Task Manager: Task {} cannot be assigned to any node.", image_task.task_uuid)).await;
                        Self::submit_image_task(image_task, false).await;
                    }
                    image_id += 1;
                }
            }
            _ => {
                let error_message = format!("Task Manager: Task {} failed because the file extension is not supported.", task.uuid);
                Self::task_panic(&task.uuid, error_message.clone()).await;
                Logger::append_system_log(LogLevel::INFO, error_message).await;
            }
        }
    }

    pub async fn redistribute_task(image_tasks: VecDeque<ImageTask>) {
        let mut current_node = 0_usize;
        for image_task in image_tasks {
            let vram_usage = TaskManager::estimated_vram_usage(&image_task.model_filepath).await;
            let filter_nodes = NodeCluster::filter_node_by_vram(vram_usage).await;
            let ram_usage = TaskManager::estimated_ram_usage(&image_task.image_filepath).await;
            let mut assigned = false;
            for i in 0..filter_nodes.len() {
                let index = (current_node + i) % filter_nodes.len();
                let node_id = match filter_nodes.get(index) {
                    Some((node_id, _)) => *node_id,
                    None => continue,
                };
                let node_ram = match NodeCluster::get_node(node_id).await {
                    Some(node) => node.read().await.idle_unused().ram,
                    None => {
                        Logger::append_system_log(LogLevel::ERROR, format!("Task Manager: Node {} does not exist.", node_id)).await;
                        continue;
                    }
                };
                if node_ram > ram_usage * 0.7 {
                    if let Some(node) = NodeCluster::get_node(node_id).await {
                        Node::add_task(node, image_task.clone()).await;
                        assigned = true;
                        current_node += 1;
                        break;
                    }
                }
            }
            if !assigned {
                Logger::append_system_log(LogLevel::WARNING, format!("Task Manager: Task {} cannot be reassigned to any node.", image_task.task_uuid)).await;
                Self::submit_image_task(image_task, false).await;
            }
        }
    }

    pub async fn task_panic(uuid: &Uuid, error: String) {
        let mut task_manager = Self::instance_mut().await;
        if let Some(mut task) = task_manager.tasks.remove(uuid) {
            task.panic(error).await;
        }
    }

    pub async fn submit_image_task(image_task: ImageTask, success: bool) {
        let uuid = image_task.task_uuid;
        let mut complete = false;
        let mut task_manager = Self::instance_mut().await;
        match task_manager.tasks.get_mut(&uuid) {
            Some(task) => {
                task.unprocessed -= 1;
                task.result.push(image_task);
                if success {
                    task.success += 1;
                } else {
                    task.failed += 1;
                }
                if task.unprocessed == 0 {
                    complete = true;
                }
            }
            None => Logger::append_system_log(LogLevel::ERROR, format!("Task Manager: Task {} does not exist.", uuid)).await,
        }
        if complete {
            if let Some(task) = task_manager.tasks.remove(&uuid) {
                FileManager::add_post_process_task(task).await;
            }
        }
    }

    pub async fn estimated_vram_usage(model_filepath: &PathBuf) -> f64 {
        let model_filesize = match fs::metadata(model_filepath).await {
            Ok(metadata) => metadata.len(),
            Err(_) => {
                Logger::append_system_log(LogLevel::ERROR, format!("Task Manager: Cannot read file {}.", model_filepath.display())).await;
                0
            }
        };
        2.4319e-6 * model_filesize as f64 + 303.3889
    }

    pub async fn estimated_ram_usage(image_filepath: &PathBuf) -> f64 {
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
