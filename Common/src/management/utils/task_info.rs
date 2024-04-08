use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::management::utils::model_type::ModelType;

#[derive(Serialize, Deserialize, Clone)]
pub struct TaskInfo {
    pub uuid: Uuid,
    pub model_filename: String,
    pub model_type: ModelType,
}

impl TaskInfo {
    pub fn new(uuid: Uuid, model_filename: String, model_type: ModelType) -> Self {
        Self {
            uuid,
            model_filename,
            model_type,
        }
    }
}
