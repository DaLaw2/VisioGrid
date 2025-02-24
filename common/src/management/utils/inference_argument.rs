use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InferenceArgument {
    pub model_type: ModelType,
    pub detect_mode: DetectMode,
    #[serde(default, skip_deserializing)]
    pub cache: bool,
    pub imgsz: usize,
    pub batch: usize,
    pub conf: f32,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum ModelType {
    Ultralytics,
    YOLOv4,
    YOLOv7,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum DetectMode {
    Predict,
    Track,
}

impl Display for DetectMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectMode::Predict => write!(f, "predict"),
            DetectMode::Track => write!(f, "track"),
        }
    }
}
