use serde::{Serialize, Deserialize};
use crate::manager::utils::bounding_box::BoundingBox;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskResult {
    pub result: Result<Vec<BoundingBox>, String>,
}

impl TaskResult {
    pub fn into(self) -> Result<Vec<BoundingBox>, String> {
        self.result
    }
}