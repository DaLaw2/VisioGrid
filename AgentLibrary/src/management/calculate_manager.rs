use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command as AsyncCommand;
use crate::utils::logging::*;
use crate::management::utils::bounding_box::BoundingBox;

pub struct CalculateManager;

impl CalculateManager {
    pub async fn ultralytics_inference(model_path: PathBuf, image_path: PathBuf) -> Result<Vec<BoundingBox>, LogEntry> {
        #[cfg(target_os = "windows")]
        let python = "python";
        #[cfg(target_os = "linux")]
        let python = "python3";
        let process = AsyncCommand::new(python)
            .arg("script/ultralytics/inference.py")
            .arg(model_path)
            .arg(image_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| error_entry!("Calculate Manager", "Unable to create process", format!("Err: {err}")))?;
        let output = process.wait_with_output().await
            .map_err(|err| error_entry!("Calculate Manager", "An error occurred during process execution", format!("Err: {err}")))?;
        if output.status.success() {
            let serialized_data = String::from_utf8_lossy(&output.stdout);
            let bounding_boxes: Vec<BoundingBox> = serde_json::from_str(&serialized_data)
                .map_err(|err| error_entry!("Calculate Manager", "Unable to deserialize data", format!("Err: {err}")))?;
            Ok(bounding_boxes)
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(error_entry!("Calculate Manager", "An error occurred during process execution", format!("Err: {err}")))?
        }
    }

    pub async fn yolov4_inference(_model_path: PathBuf, _image_path: PathBuf) -> Result<Vec<BoundingBox>, LogEntry> {
        Ok(Vec::new())
    }

    pub async fn yolov7_inference(_model_path: PathBuf, _image_path: PathBuf) -> Result<Vec<BoundingBox>, LogEntry> {
        Ok(Vec::new())
    }
}
