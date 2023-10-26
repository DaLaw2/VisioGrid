use std::str::FromStr;
use crate::manager::task_manager::TaskManager;

pub struct Task {
    pub uuid: usize,
    pub status: TaskStatus,
    pub processed: usize,
    pub unprocessed: usize,
    pub ip: String,
    pub model_filename: String,
    pub inference_filename: String,
    pub inference_type: InferenceType,
}

impl Task {
    pub async fn new(ip: String, model_filename: String, inference_filename: String, inference_type: InferenceType) -> Self {
        Self {
            uuid: TaskManager::allocate_uuid().await,
            status: TaskStatus::PreProcessing,
            processed: 0_usize,
            unprocessed: 0_usize,
            ip,
            model_filename,
            inference_filename,
            inference_type,
        }
    }
}

pub enum TaskStatus {
    PreProcessing,
    Waiting,
    Processing,
    PostProcessing,
    Fail,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum InferenceType {
    YOLO,
    PyTorch,
    TensorFlow,
    ONNX,
    Default,
}

impl FromStr for InferenceType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "YOLO" => Ok(InferenceType::YOLO),
            "PyTorch" => Ok(InferenceType::PyTorch),
            "TensorFlow" => Ok(InferenceType::TensorFlow),
            "ONNX" => Ok(InferenceType::ONNX),
            _ => Ok(InferenceType::Default),
        }
    }
}