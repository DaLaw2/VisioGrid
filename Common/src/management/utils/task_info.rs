use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::management::utils::inference_type::InferenceType;

#[derive(Serialize, Deserialize, Clone)]
pub struct TaskInfo {
    uuid: Uuid,
    model_filename: String,
    inference_type: InferenceType,
}

impl TaskInfo {
    pub fn new(uuid: Uuid, model_filename: String, inference_type: InferenceType) -> Self {
        Self {
            uuid,
            model_filename,
            inference_type,
        }
    }
}
