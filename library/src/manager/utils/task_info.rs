use uuid::Uuid;
use std::fmt::{self, Display, Formatter};
use crate::manager::utils::image_task::ImageTask;
use crate::manager::utils::inference_type::InferenceType;

#[derive(Debug, Clone)]
pub struct TaskInfo {
    uuid: Uuid,
    model_filename: String,
    inference_type: InferenceType,
}

impl TaskInfo {
    pub fn new(task: &ImageTask) -> Self {
        Self {
            uuid: task.task_uuid,
            model_filename: task.model_filename.clone(),
            inference_type: task.inference_type,
        }
    }
}

impl Display for TaskInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{}", self.uuid, self.model_filename, self.inference_type.to_string())
    }
}
