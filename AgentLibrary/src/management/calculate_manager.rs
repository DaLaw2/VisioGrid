use std::path::PathBuf;
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};
use tch::{Device, nn, vision, CModule, Tensor};
use Common::utils::logger::LogLevel;
use crate::utils::logger::LogEntry;
use crate::management::utils::bounding_box::BoundingBox;
use crate::utils::config::Config;

pub struct CalculateManager;

impl CalculateManager {
    fn new() -> Self {
        Self
    }

    async fn inference(model_path: PathBuf, image_path: PathBuf) -> Result<Vec<BoundingBox>, LogEntry> {

    }

}
