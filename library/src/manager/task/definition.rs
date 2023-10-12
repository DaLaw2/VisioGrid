use std::str::FromStr;

pub struct Task {
    pub ip: String,
    pub model_filename: String,
    pub inference_filename: String,
    pub inference_type: InferenceType,
    pub processed: usize,
    pub unprocessed: usize,
    pub status: TaskStatus,
}

impl Task {
    pub fn new(ip: String, model_filename: String, inference_filename: String, inference_type: InferenceType) -> Self {
        Self {
            ip,
            model_filename,
            inference_filename,
            inference_type,
            processed: 0_usize,
            unprocessed: 0_usize,
            status: TaskStatus::PreProcessing,
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

#[derive(Hash, Eq, PartialEq, Ord, PartialOrd)]
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
