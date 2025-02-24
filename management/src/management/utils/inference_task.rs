use crate::management::utils::inference_argument::InferenceArgument;
use crate::management::utils::task::Task;
use crate::management::utils::task_info::TaskInfo;
use std::path::PathBuf;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize, Debug, Clone)]
pub struct InferenceTask {
    pub task_uuid: Uuid,
    pub model_file_name: String,
    pub media_file_name: String,
    pub model_file_path: PathBuf,
    pub media_file_path: PathBuf,
    pub inference_argument: InferenceArgument,
    pub error: Result<(), String>,
}

impl InferenceTask {
    pub fn new(task: &Task, model_file_path: PathBuf, media_file_path: PathBuf) -> Self {
        let image_file_name = media_file_path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default().to_string();
        Self {
            task_uuid: task.uuid,
            model_file_name: task.model_file_name.clone(),
            media_file_name: image_file_name,
            model_file_path,
            media_file_path,
            inference_argument: task.inference_argument.clone(),
            error: Ok(())
        }
    }

    pub fn as_task_info(&self) -> TaskInfo {
        TaskInfo::new(self.task_uuid, self.model_file_name.clone(), self.media_file_name.clone(), self.inference_argument.clone())
    }
}
