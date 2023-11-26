use uuid::Uuid;
use std::path::PathBuf;
use crate::manager::utils::bounding_box::BoundingBox;
use crate::manager::utils::inference_type::InferenceType;

#[derive(Debug, Clone)]
pub struct ImageResource {
    pub task_uuid: Uuid,
    pub model_filename: String,
    pub image_filename: String,
    pub model_filepath: PathBuf,
    pub image_filepath: PathBuf,
    pub inference_type: InferenceType,
    pub bounding_boxes: Vec<BoundingBox>,
    pub cache: bool,
}

impl ImageResource {
    pub fn new(task_uuid: Uuid, model_filepath: PathBuf, image_filepath: PathBuf, inference_type: InferenceType) -> Self {
        let model_filename = model_filepath.clone()
            .file_name().and_then(|name| name.to_str())
            .unwrap_or_default().to_string();
        let image_filename = image_filepath.clone()
            .file_name().and_then(|name| name.to_str())
            .unwrap_or_default().to_string();
        Self {
            task_uuid,
            model_filename,
            image_filename,
            model_filepath,
            image_filepath,
            inference_type,
            bounding_boxes: Vec::new(),
            cache: false,
        }
    }
}
