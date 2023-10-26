use tokio::fs;
use std::path::Path;
use tokio::time::sleep;
use tokio::sync::Mutex;
use std::time::Duration;
use lazy_static::lazy_static;
use std::collections::VecDeque;
use priority_queue::PriorityQueue;
use crate::manager::definition::Task;
use crate::utils::id_manager::IDManager;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::node_cluster::NodeCluster;
use crate::manager::utils::infeerence_resource::InferenceResource;

lazy_static! {
    static ref TASK_UUID_MANAGER: Mutex<IDManager> = Mutex::new(IDManager::new());
    static ref GLOBAL_TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}

pub struct TaskManager {
    task_queue: VecDeque<Task>,
    //這裡要更改
    process_queue: PriorityQueue<InferenceResource, (usize, usize)>
}

impl TaskManager {
    fn new() -> Self {
        Self {
            task_queue: VecDeque::new(),
            process_queue: PriorityQueue::new(),
        }
    }

    pub async fn allocate_uuid() -> usize {
        let mut id_manager = TASK_UUID_MANAGER.lock().await;
        id_manager.allocate_id()
    }

    pub async fn free_uuid(uuid: usize) {
        let mut id_manager = TASK_UUID_MANAGER.lock().await;
        id_manager.free_id(uuid)
    }

    pub async fn add_task(task: Task) {
        let mut task_manager = GLOBAL_TASK_MANAGER.lock().await;
        task_manager.task_queue.push_back(task);
    }

    async fn add_inference_resource() {
        loop {
            let mut task_manager = GLOBAL_TASK_MANAGER.lock().await;
            let node_amount = { NodeCluster::instance().await.size() };
            while task_manager.process_queue.len() < node_amount {
                let mut process: Vec<(InferenceResource, (usize, usize))> = Vec::new();
                match task_manager.task_queue.front_mut() {
                    Some(task) => {
                        match Path::new(&task.inference_filename).extension().and_then(|os_str| os_str.to_str()) {
                            Some("png") | Some("jpg") | Some("jpeg") => {
                                let model_filepath = Path::new(".").join("SavedModel").join(task.model_filename.clone());
                                let inference_filepath = Path::new(".").join("PreProcessing").join(task.inference_filename.clone());
                                let inference_resource = InferenceResource::new(task.uuid, model_filepath, inference_filepath, task.inference_type.clone()).await;
                                let priority = Self::calc_priority(&inference_resource).await;
                                process.push((inference_resource, priority));
                            },
                            Some("gif") | Some("mp4") | Some("wav") | Some("avi") | Some("mkv") | Some("zip") => {
                                let model_filepath = Path::new(".").join("SavedModel").join(task.model_filename.clone());
                                let inference_folder = Path::new(".").join("PreProcessing").join(task.inference_filename.clone()).with_extension("");
                                let mut inference_folder = match fs::read_dir(&inference_folder).await {
                                    Ok(inference_folder) => inference_folder,
                                    Err(err) => {
                                        Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Fail read folder {}: {:?}", inference_folder.display(), err));
                                        continue;
                                    }
                                };
                                while let Ok(Some(inference_filepath)) = inference_folder.next_entry().await {
                                    let inference_filepath = inference_filepath.path();
                                    let inference_resource = InferenceResource::new(task.uuid, model_filepath.clone(), inference_filepath, task.inference_type.clone()).await;
                                    let priority = Self::calc_priority(&inference_resource).await;
                                    process.push((inference_resource, priority));
                                }
                            },
                            _ => Logger::instance().await.append_global_log(LogLevel::ERROR, "Add image to task manager failed.".to_string()),
                        }
                    },
                    None => break
                }
                for (inference_resource, priority) in process {
                    task_manager.process_queue.push(inference_resource, priority);
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn run() {
        tokio::spawn(async {
            Self::add_inference_resource().await;
        });
    }

    async fn calc_priority(inference_resource: &InferenceResource) -> (usize, usize) {
        let inference_filesize = match fs::metadata(&inference_resource.inference_filepath).await {
            Ok(metadata) => metadata.len(),
            Err(err) => {
                Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Fail read file {}: {:?}", inference_resource.inference_filepath.display(), err));
                0
            }
        };
        let model_filesize = match fs::metadata(&inference_resource.model_filepath).await {
            Ok(metadata) => metadata.len(),
            Err(err) => {
                Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Fail read file {}: {:?}", inference_resource.inference_filepath.display(), err));
                0
            }
        };
        (model_filesize as usize, inference_filesize as usize)
    }

    fn distributed_processing() {

    }
}
