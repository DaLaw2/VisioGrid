use tokio::sync::RwLock;
use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::ffi::OsStr;
use tokio::fs;
use std::path::Path;
use crate::manager::utils::task::Task;
use crate::utils::logger::{Logger, LogLevel};

lazy_static! {
    static ref GLOBAL_RESULT_REPOSITORY: RwLock<ResultRepository> = RwLock::new(ResultRepository::new());
}

pub struct ResultRepository {
    success: VecDeque<Task>,
    fail: VecDeque<Task>
}

impl ResultRepository {
    fn new() -> Self {
        Self {
            success: VecDeque::new(),
            fail: VecDeque::new(),
        }
    }

    async fn cleanup(task: &Task) {

    }

    pub async fn task_failed(task: Task) {
        let mut result_repository = GLOBAL_RESULT_REPOSITORY.write().await;
        Self::cleanup(&task).await;
        result_repository.fail.push_back(task);
    }
}
