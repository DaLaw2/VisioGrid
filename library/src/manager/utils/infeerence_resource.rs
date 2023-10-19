use std::path::PathBuf;
use crate::manager::task::definition::InferenceType;
use crate::manager::utils::bounding_box::BoundingBox;

#[derive(Hash, Eq, PartialEq)]
pub struct InferenceResource {
    pub task_uuid: usize,
    pub model_filepath: PathBuf,
    pub inference_filepath: PathBuf,
    pub inference_type: InferenceType,
    pub bounding_boxes: Vec<BoundingBox>,
}

impl InferenceResource {
    pub async fn new(task_uuid: usize, model_filepath: PathBuf, inference_filepath: PathBuf, inference_type: InferenceType) -> Self {
        Self {
            task_uuid,
            model_filepath,
            inference_filepath,
            inference_type,
            bounding_boxes: Vec::new(),
        }
    }
}
