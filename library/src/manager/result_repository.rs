use tokio::sync::RwLock;
use lazy_static::lazy_static;
use std::collections::VecDeque;
use crate::manager::utils::task::Task;

lazy_static! {
    static ref GLOBAL_RESULT_REPOSITORY: RwLock<ResultRepository> = RwLock::new(ResultRepository::new());
}

pub struct ResultRepository {
    result: VecDeque<Task>
}

impl ResultRepository {
    fn new() -> Self {
        Self {
            result: VecDeque::new(),
        }
    }

    pub async fn add_task(task: Task) {
        let result_repository = GLOBAL_RESULT_REPOSITORY.write().await;
        result_repository.result.push_back(task);
    }
}
