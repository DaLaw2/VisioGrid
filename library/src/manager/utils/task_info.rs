use uuid::Uuid;
use std::fmt::{self, Display, Formatter};
use crate::manager::utils::inference_type::InferenceType;
use crate::manager::utils::image_resource::ImageResource;

#[derive(Debug, Clone)]
pub struct TaskInfo {
    uuid: Uuid,
    model_filename: String,
    inference_type: InferenceType,
}

impl TaskInfo {
    pub fn new(task: &ImageResource) -> Self {
        Self {
            uuid: task.task_uuid,
            model_filename: task.model_filename.clone(),
            inference_type: task.inference_type,
        }
    }
}

impl Display for TaskInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} Model FileName: {}, Inference Type: {}", self.uuid, self.model_filename, self.inference_type.to_string())
    }
}
