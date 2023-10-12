use std::str::FromStr;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::task::definition::InferenceType;
use crate::manager::utils::bounding_box::BoundingBox;

#[derive(Hash, Eq, PartialEq)]
pub struct ImageResource {
    file_name: String,
    file_path: String,
    image_size: usize,
    inference_type: InferenceType,
    bounding_boxes: Vec<BoundingBox>
}

impl ImageResource {
    pub async fn new(file_name: String, file_path: String) -> Self {
        let parts: Vec<&str> = file_name.split('_').collect();
        let inference_type = match parts.get(1) {
            Some(str) => InferenceType::from_str(str).unwrap(),
            None => {
                Logger::instance().await.append_global_log(LogLevel::ERROR, format!("Fail parse file name: {}.", file_name));
                InferenceType::Default
            }
        };
        Self {
            file_name,
            file_path,
            image_size: 0,
            inference_type,
            bounding_boxes: Vec::new(),
        }
    }
}
