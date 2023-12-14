use uuid::Uuid;
use std::path::PathBuf;
use crate::manager::utils::task::Task;
use crate::manager::utils::bounding_box::BoundingBox;
use crate::manager::utils::inference_type::InferenceType;

#[derive(Debug, Clone)]
pub struct ImageTask {
    pub id: usize,
    pub task_uuid: Uuid,
    pub model_filename: String,
    pub image_filename: String,
    pub model_filepath: PathBuf,
    pub image_filepath: PathBuf,
    pub inference_type: InferenceType,
    pub bounding_boxes: Vec<BoundingBox>,
    pub cache: bool,
}

impl ImageTask {
    pub fn new(id: usize, task: &Task, model_filepath: PathBuf, image_filepath: PathBuf) -> Self {
        let image_filename = image_filepath.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default().to_string();
        Self {
            id,
            task_uuid: task.uuid,
            model_filename: task.model_filename.clone(),
            image_filename,
            model_filepath,
            image_filepath,
            inference_type: task.inference_type,
            bounding_boxes: Vec::new(),
            cache: false,
        }
    }
}
