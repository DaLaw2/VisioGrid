use tokio::fs;
use std::path::Path;
use tokio::sync::RwLock;
use lazy_static::lazy_static;
use std::collections::VecDeque;
use crate::manager::utils::task::Task;

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
        let model_filepath = Path::new(".").join("SavedModel").join(task.model_filename.clone());
        let pre_process_media = Path::new(".").join("PreProcessing").join(task.media_filename.clone());
        let pre_process_folder = pre_process_media.with_extension("");
        let post_process_media = Path::new(".").join("PostProcessing").join(task.media_filename.clone());
        let post_process_folder = post_process_media.with_extension("");
        let result_filepath = Path::new(".").join("Result").join(task.media_filename.clone());
        let _ = fs::rename(&post_process_media, &result_filepath).await;
        let _ = fs::remove_file(model_filepath).await;
        let _ = fs::remove_dir_all(pre_process_folder).await;
        let _ = fs::remove_dir_all(post_process_folder).await;
    }

    pub async fn task_success(task: Task) {
        let mut result_repository = GLOBAL_RESULT_REPOSITORY.write().await;
        Self::cleanup(&task).await;
        result_repository.success.push_back(task);
    }

    pub async fn task_failed(task: Task) {
        let mut result_repository = GLOBAL_RESULT_REPOSITORY.write().await;
        Self::cleanup(&task).await;
        result_repository.fail.push_back(task);
    }
}
