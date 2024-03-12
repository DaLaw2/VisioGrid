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
        let config = Config::now().await;
        let model = CModule::load(model_path)
            .map_err(|_| LogEntry::new(LogLevel::ERROR, "Calculate Manager: Fail to load the model.".to_string()))?;
        let image = vision::image::load(image_path)
            .map_err(|_| LogEntry::new(LogLevel::ERROR, "Calculate Manager: Failed to load the image.".to_string()))?
            .to(Device::Cuda(0));
        let image = image.unsqueeze(0);
        let mut output = model.forward_ts(&[image])
            .map_err(|_| LogEntry::new(LogLevel::ERROR, "Calculate Manager: Failed during the inference.".to_string()))?;
        let detections = output.squeeze_dim(0);
        let conf_mask = detections.select(1, 4).gt(config.confidence_threshold);
        let detections = detections.boolean_mask(&conf_mask, 0);
    }

    #[allow(non_snake_case)]
    fn NMS() {

    }
}
