use uuid::Uuid;
use std::path::PathBuf;
use crate::management::utils::task::Task;
use crate::management::utils::bounding_box::BoundingBox;
use crate::management::utils::model_type::ModelType;

#[derive(Debug, Clone)]
pub struct ImageTask {
    pub id: usize,
    pub task_uuid: Uuid,
    pub model_filename: String,
    pub image_filename: String,
    pub model_filepath: PathBuf,
    pub image_filepath: PathBuf,
    pub model_type: ModelType,
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
            model_type: task.model_type,
            bounding_boxes: Vec::new(),
            cache: false,
        }
    }
}
