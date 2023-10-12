use std::sync::Arc;
use tokio::sync::Mutex;
use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use priority_queue::PriorityQueue;
use crate::manager::node_cluster::NodeCluster;
use crate::manager::task::definition::{Task, TaskStatus};
use crate::manager::utils::image_resource::ImageResource;
use crate::utils::logger::{Logger, LogLevel};

lazy_static! {
    static ref GLOBAL_TASK_MANAGER: Arc<Mutex<TaskManager>> = Arc::new(Mutex::new(TaskManager::new()));
}

pub struct TaskManager {
    task_queue: VecDeque<Task>,
    image_queue: PriorityQueue<ImageResource, usize>
}

impl TaskManager {
    fn new() -> Self {
        Self {
            task_queue: VecDeque::new(),
            image_queue: PriorityQueue::new(),
        }
    }

    pub async fn add_task(task: Task) {
        let mut manager = GLOBAL_TASK_MANAGER.lock().await;
        manager.task_queue.push_back(task);
    }

    //當圖片數小於node數
    //將任務的image丟到queue裡面
    //最少一個task進入
    pub async fn run() {
        loop {
            let mut task_manager = GLOBAL_TASK_MANAGER.lock().await;
            let node_amount = { NodeCluster::instance().await.size() };
            while task_manager.image_queue.len() < node_amount {
                match task_manager.task_queue.front_mut() {
                    Some(task) => {
                        task.status = TaskStatus::Processing;
                        match Path::new(&task.inference_filename).extension().and_then(|os_str| os_str.to_str()) {
                            Some("png") | Some("jpg") | Some("jpeg") => {

                            },
                            Some("gif") | Some("mp4") | Some("wav") | Some("avi") | Some("mkv") | Some("zip") => {

                            },
                            _ => Logger::instance().await.append_global_log(LogLevel::ERROR, "Add image to task manager failed.".to_string()),
                        }
                    },
                    None => break
                }
            }
        }
    }
}