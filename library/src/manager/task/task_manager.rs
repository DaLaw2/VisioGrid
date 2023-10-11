use std::sync::Arc;
use tokio::sync::Mutex;
use lazy_static::lazy_static;
use std::collections::VecDeque;
use crate::manager::task::definition::Task;
use crate::manager::utils::image_resource::ImageResource;

lazy_static!{
    static ref GLOBAL_TASK_MANAGER: Arc<Mutex<TaskManager>> = Arc::new(Mutex::new(TaskManager::new()));
}

pub struct TaskManager {
    task_queue: VecDeque<Task>,
    // image_queue: BinaryHeap<(usize, ImageResource)>,
}

impl TaskManager {
    fn new() -> Self {
        Self {
            task_queue: VecDeque::new(),
            // image_queue: BinaryHeap::new(),
        }
    }

    pub async fn add_task(task: Task) {
        let mut manager = GLOBAL_TASK_MANAGER.lock().await;
        manager.task_queue.push_back(task);
    }
}
