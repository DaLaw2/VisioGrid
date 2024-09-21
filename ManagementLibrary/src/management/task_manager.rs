use crate::management::agent::Agent;
use crate::management::agent_manager::AgentManager;
use crate::management::media_processor::MediaProcessor;
use crate::management::utils::inference_task::InferenceTask;
use crate::management::utils::task::{Task, TaskStatus};
use crate::utils::config::{Config, SplitMode};
use crate::utils::logging::*;
use lazy_static::lazy_static;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

lazy_static! {
    static ref TASK_MANAGER: RwLock<TaskManager> = RwLock::new(TaskManager::new());
}

pub struct TaskManager {
    success: VecDeque<Task>,
    fail: VecDeque<Task>,
    processing: HashMap<Uuid, Task>,
}

impl TaskManager {
    fn new() -> Self {
        Self {
            success: VecDeque::new(),
            fail: VecDeque::new(),
            processing: HashMap::new(),
        }
    }

    pub async fn instance() -> RwLockReadGuard<'static, Self> {
        TASK_MANAGER.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        TASK_MANAGER.write().await
    }

    pub async fn add_task(task: Task) {
        let mut task_manager = Self::instance_mut().await;
        task_manager.processing.insert(task.uuid, task.clone());
        drop(task_manager);
        MediaProcessor::add_pre_process_task(task).await;
    }

    pub async fn clone_processing_task(uuid: &Uuid) -> Option<Task> {
        let task_manager = Self::instance().await;
        task_manager.processing.get(uuid).cloned()
    }

    pub async fn get_processing_tasks() -> Vec<Task> {
        let task_manager = Self::instance().await;
        task_manager.processing.values().cloned().collect()
    }

    pub async fn clone_success_task(uuid: &Uuid) -> Option<Task> {
        let task_manager = Self::instance().await;
        task_manager.success.iter().find(|task| task.uuid == *uuid).cloned()
    }

    pub async fn get_success_tasks() -> Vec<Task> {
        let task_manager = Self::instance().await;
        task_manager.success.iter().cloned().collect()
    }

    pub async fn clone_fail_task(uuid: &Uuid) -> Option<Task> {
        let task_manager = Self::instance().await;
        task_manager.fail.iter().find(|task| task.uuid == *uuid).cloned()
    }

    pub async fn get_fail_tasks() -> Vec<Task> {
        let task_manager = Self::instance().await;
        task_manager.fail.iter().cloned().collect()
    }

    pub async fn task_success(uuid: &Uuid) {
        let mut task_manager = Self::instance_mut().await;
        let task = if let Some(mut task) = task_manager.processing.remove(uuid) {
            task.status = TaskStatus::Success;
            task_manager.success.push_back(task.clone());
            task
        } else {
            return;
        };
        Self::task_cleanup(&task).await;
    }

    pub async fn task_failed(uuid: &Uuid, error_message: String) {
        let mut task_manager = Self::instance_mut().await;
        let task = if let Some(mut task) = task_manager.processing.remove(uuid) {
            task.status = TaskStatus::Fail;
            task.error = Err(error_message);
            task_manager.fail.push_back(task.clone());
            task
        } else {
            return;
        };
        Self::task_cleanup(&task).await;
    }

    pub async fn task_cleanup(task: &Task) {
        let uuid = task.uuid.to_string();
        #[cfg(target_os = "linux")]
        let model_file_path = PathBuf::from(format!("./SavedModel/{}", task.model_file_name));
        #[cfg(target_os = "windows")]
        let model_file_path = PathBuf::from(format!(".\\SavedModel\\{}", task.model_file_name));
        #[cfg(target_os = "linux")]
        let pre_process_folder = PathBuf::from(format!("./PreProcess/{}", uuid));
        #[cfg(target_os = "windows")]
        let pre_process_folder = PathBuf::from(format!(".\\PreProcess\\{}", uuid));
        #[cfg(target_os = "linux")]
        let post_process_folder = PathBuf::from(format!("./PostProcess/{}", uuid));
        #[cfg(target_os = "windows")]
        let post_process_folder = PathBuf::from(format!(".\\PostProcess\\{}", uuid));
        let _ = fs::remove_file(model_file_path).await;
        let _ = fs::remove_dir_all(pre_process_folder).await;
        let _ = fs::remove_dir_all(post_process_folder).await;
    }

    pub async fn change_task_status(uuid: &Uuid, status: TaskStatus) {
        let mut task_manager = Self::instance_mut().await;
        if let Some(task) = task_manager.processing.get_mut(uuid) {
            task.status = status;
        }
    }

    pub async fn update_unprocessed(uuid: &Uuid, unprocessed: usize) {
        let mut task_manager = Self::instance_mut().await;
        if let Some(task) = task_manager.processing.get_mut(uuid) {
            task.unprocessed = unprocessed;
        }
    }

    pub async fn distribute_task(task: Task) {
        TaskManager::change_task_status(&task.uuid, TaskStatus::Processing).await;
        match Path::new(&task.media_file_name).extension().and_then(|os_str| os_str.to_str()) {
            Some("png") | Some("jpg") | Some("jpeg") => Self::distribute_image(task).await,
            Some("mp4") | Some("avi") | Some("mkv") | Some("zip") => Self::distribute_video_and_zip(task).await,
            _ => {
                let error_message = TaskEntry::UnSupportFileType(task.uuid);
                TaskManager::task_failed(&task.uuid, error_message.to_string()).await;
                information_entry!(error_message);
            }
        }
    }

    async fn distribute_image(task: Task) {
        let task_uuid = task.uuid.to_string();
        #[cfg(target_os = "linux")]
        let model_file_path = PathBuf::from(format!("./SavedModel/{}", task.model_file_name));
        #[cfg(target_os = "windows")]
        let model_file_path = PathBuf::from(format!(".\\SavedModel\\{}", task.model_file_name));
        #[cfg(target_os = "linux")]
        let image_file_path = PathBuf::from(format!("./PreProcess/{}/{}", task_uuid, task.media_file_name));
        #[cfg(target_os = "windows")]
        let image_file_path = PathBuf::from(format!(".\\PreProcess\\{}\\{}", task_uuid, task.media_file_name));
        let estimated_vram_usage = Self::estimated_vram_usage(&model_file_path).await;
        let estimated_ram_usage = Self::estimated_ram_usage(&image_file_path).await;
        let filter_agents = AgentManager::filter_agent_by_vram(estimated_vram_usage).await;
        let mut inference_task = InferenceTask::new(&task, model_file_path, image_file_path);
        for (agent_uuid, _) in filter_agents {
            let ram = AgentManager::get_agent_unused_ram(agent_uuid).await.unwrap_or(0.0);
            if ram > estimated_ram_usage * 0.7 {
                if let Some(agent) = AgentManager::get_agent(agent_uuid).await {
                    if ram < estimated_ram_usage {
                        inference_task.inference_argument.cache = true;
                    }
                    Agent::add_task(agent, inference_task).await;
                    return;
                }
            }
        }
        let error_message = TaskEntry::TaskAssignError(task.uuid);
        inference_task.error = Err(error_message.to_string());
        logging_warning!(error_message);
        Self::submit_inference_task(inference_task).await;
    }

    pub async fn distribute_video_and_zip(task: Task) {
        let mut current_agent = 0_usize;
        let uuid = task.uuid.to_string();
        let config = Config::now().await;
        #[cfg(target_os = "linux")]
        let model_file_path = PathBuf::from(format!("./SavedModel/{}", task.model_file_name));
        #[cfg(target_os = "windows")]
        let model_file_path = PathBuf::from(format!(".\\SavedModel\\{}", task.model_file_name));
        #[cfg(target_os = "linux")]
        let media_folder = PathBuf::from(format!("./PreProcess/{}", uuid));
        #[cfg(target_os = "windows")]
        let media_folder = PathBuf::from(format!(".\\PreProcess\\{}", uuid));
        let ignore_file1 = media_folder.join(&task.media_file_name);
        let ignore_file2 = ignore_file1.with_extension("toml");
        let estimated_vram_usage = Self::estimated_vram_usage(&model_file_path).await;
        let filter_agents = AgentManager::filter_agent_by_vram(estimated_vram_usage).await;
        let mut media_folder = match fs::read_dir(&media_folder).await {
            Ok(media_folder) => media_folder,
            Err(err) => {
                let error_message = IOEntry::ReadDirectoryError(media_folder.display(), err);
                Self::task_failed(&task.uuid, error_message.to_string()).await;
                logging_error!(error_message);
                return;
            }
        };
        'outer: while let Ok(Some(dir_entry)) = media_folder.next_entry().await {
            let media_file_path = dir_entry.path();
            if media_file_path == ignore_file1 || media_file_path == ignore_file2 {
                continue;
            }
            let ram_usage = match config.split_mode {
                SplitMode::Frame => Self::estimated_ram_usage(&media_file_path).await,
                SplitMode::Time { .. } => {
                    let batch_size = task.inference_argument.batch;
                    Self::estimated_ram_usage(&media_file_path).await * (batch_size as f64)
                }
            };
            let mut inference_task = InferenceTask::new(&task, model_file_path.clone(), media_file_path.clone());
            for i in 0..filter_agents.len() {
                let index = (current_agent + i) % filter_agents.len();
                let agent_uuid = match filter_agents.get(index) {
                    Some((agent_uuid, _)) => *agent_uuid,
                    None => continue,
                };
                let agent_ram = AgentManager::get_agent_unused_ram(agent_uuid).await.unwrap_or(0.0);
                if agent_ram > ram_usage * 0.7 {
                    if let Some(agent) = AgentManager::get_agent(agent_uuid).await {
                        if agent_ram < ram_usage {
                            inference_task.inference_argument.cache = true;
                        }
                        Agent::add_task(agent, inference_task).await;
                        current_agent += 1;
                        continue 'outer;
                    }
                }
            }
            let error_message = TaskEntry::TaskAssignError(task.uuid);
            inference_task.error = Err(error_message.to_string());
            logging_warning!(error_message);
            Self::submit_inference_task(inference_task).await;
        }
    }

    pub async fn redistribute_task(inference_tasks: VecDeque<InferenceTask>) {
        let mut current_agent = 0_usize;
        'outer: for mut inference_task in inference_tasks {
            let estimated_ram_usage = TaskManager::estimated_ram_usage(&inference_task.media_file_path).await;
            let estimated_vram_usage = TaskManager::estimated_vram_usage(&inference_task.model_file_path).await;
            let filter_agents = AgentManager::filter_agent_by_vram(estimated_vram_usage).await;
            for i in 0..filter_agents.len() {
                let index = (current_agent + i) % filter_agents.len();
                let agent_uuid = match filter_agents.get(index) {
                    Some((agent_id, _)) => *agent_id,
                    None => continue,
                };
                let ram = AgentManager::get_agent_unused_ram(agent_uuid).await.unwrap_or(0.0);
                if ram > estimated_ram_usage * 0.7 {
                    if let Some(agent) = AgentManager::get_agent(agent_uuid).await {
                        Agent::add_task(agent, inference_task).await;
                        current_agent += 1;
                        continue 'outer;
                    }
                }
            }
            let error_message = TaskEntry::TaskAssignError(inference_task.task_uuid);
            inference_task.error = Err(error_message.to_string());
            logging_warning!(error_message);
            Self::submit_inference_task(inference_task).await;
        }
    }

    pub async fn steal_task(agent: Arc<RwLock<Agent>>) -> Option<InferenceTask> {
        let agents = AgentManager::sorted_by_vram().await;
        let (vram, ram) = {
            let agent = agent.read().await;
            let idle_unused = agent.idle_unused();
            (idle_unused.vram, idle_unused.ram)
        };
        for (agent_uuid, _) in agents {
            if let Some(agent) = AgentManager::get_agent(agent_uuid).await {
                let mut steal = false;
                let mut cache = false;
                let mut agent = agent.write().await;
                if let Some(inference_task) = agent.inference_tasks().get(0) {
                    let estimate_ram = TaskManager::estimated_ram_usage(&inference_task.media_file_path).await;
                    let estimate_vram = TaskManager::estimated_vram_usage(&inference_task.model_file_path).await;
                    if ram > estimate_ram * 0.7 && vram > estimate_vram {
                        steal = true;
                        if ram < estimate_ram {
                            cache = true;
                        }
                    }
                }
                if steal {
                    if let Some(mut inference_task) = agent.inference_tasks().pop_front() {
                        inference_task.inference_argument.cache = cache;
                        return Some(inference_task);
                    }
                }
            }
        }
        None
    }

    pub async fn submit_inference_task(inference_task: InferenceTask) {
        let uuid = inference_task.task_uuid;
        let mut task_manager = Self::instance_mut().await;
        match task_manager.processing.get_mut(&uuid) {
            Some(task) => {
                let success = inference_task.error.is_ok();
                task.unprocessed -= 1;
                task.result.push(inference_task);
                if success {
                    task.success += 1;
                } else {
                    task.failed += 1;
                }
                if task.unprocessed == 0 {
                    task.status = TaskStatus::Waiting;
                    MediaProcessor::add_post_process_task(task.clone()).await;
                }
            }
            None => logging_error!(TaskEntry::TaskDoesNotExist(uuid)),
        }
    }

    pub async fn estimated_vram_usage(model_file_path: &PathBuf) -> f64 {
        let model_filesize = match fs::metadata(model_file_path).await {
            Ok(metadata) => metadata.len(),
            Err(err) => {
                logging_error!(IOEntry::ReadFileError(model_file_path.display(), err));
                0
            }
        };
        2.4319e-6 * model_filesize as f64 + 303.3889
    }

    pub async fn estimated_ram_usage(media_file_path: &PathBuf) -> f64 {
        let media_filesize = match fs::metadata(media_file_path).await {
            Ok(metadata) => metadata.len(),
            Err(err) => {
                logging_error!(IOEntry::ReadFileError(media_file_path.display(), err));
                0
            }
        };
        4.1894 * media_filesize as f64 + 1_398_237_298.688
    }
}
