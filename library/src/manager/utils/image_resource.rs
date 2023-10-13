use std::path::PathBuf;
use tokio::fs::metadata;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::task::definition::InferenceType;
use crate::manager::utils::bounding_box::BoundingBox;

#[derive(Hash, Eq, PartialEq)]
pub struct ImageResource {
    task_uuid: usize,
    model_filepath: PathBuf,
    inference_filepath: PathBuf,
    image_filesize: usize,
    inference_type: InferenceType,
    bounding_boxes: Vec<BoundingBox>,
}

impl ImageResource {
    pub async fn new(task_uuid: usize, model_filepath: PathBuf, inference_filepath: PathBuf, inference_type: InferenceType) -> Self {
        let image_filesize = match metadata(&inference_filepath).await {
            Ok(metadata) => metadata.len() as usize,
            Err(_) => {
                Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Fail read file {:?}.", inference_filepath.file_name().unwrap_or_default()));
                usize::MAX
            }
        };
        Self {
            task_uuid,
            model_filepath,
            inference_filepath,
            image_filesize,
            inference_type,
            bounding_boxes: Vec::new(),
        }
    }
}
