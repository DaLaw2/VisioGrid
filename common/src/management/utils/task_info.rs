use crate::management::utils::inference_argument::InferenceArgument;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct TaskInfo {
    pub uuid: Uuid,
    pub model_file_name: String,
    pub media_file_name: String,
    pub inference_argument: InferenceArgument,
}

impl TaskInfo {
    pub fn new(uuid: Uuid, model_file_name: String, image_file_name: String, inference_argument: InferenceArgument) -> Self {
        Self {
            uuid,
            model_file_name,
            media_file_name: image_file_name,
            inference_argument,
        }
    }
}
