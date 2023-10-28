use uuid::Uuid;
use std::path::PathBuf;
use crate::manager::utils::inference_type::InferenceType;
use crate::manager::utils::bounding_box::BoundingBox;

#[derive(Hash, Eq, PartialEq)]
pub struct ImageResource {
    pub task_uuid: Uuid,
    pub model_filepath: PathBuf,
    pub image_filepath: PathBuf,
    pub inference_type: InferenceType,
    pub bounding_boxes: Vec<BoundingBox>,
}

impl ImageResource {
    pub fn new(task_uuid: Uuid, model_filepath: PathBuf, image_filepath: PathBuf, inference_type: InferenceType) -> Self {
        Self {
            task_uuid,
            model_filepath,
            image_filepath,
            inference_type,
            bounding_boxes: Vec::new(),
        }
    }
}
