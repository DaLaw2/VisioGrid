#[derive(Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum InferenceType {
    YOLO,
    PyTorch,
    TensorFlow,
    ONNX,
    Default
}

#[derive(Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum TaskStatus {
    PreProcessing,
    Waiting,
    Processing,
    PostProcessing,
    Fail
}

#[derive(Hash, Eq, PartialEq,Ord, PartialOrd)]
pub struct Task {
    pub ip: String,
    pub status: TaskStatus,
    pub model_filename: String,
    pub inference_filename: String,
    pub inference_type: InferenceType,
    pub processed: usize,
    pub unprocessed: usize,
}

impl Task {
    pub fn new(ip: String, model_filename: String, inference_filename: String, inference_type: InferenceType) -> Self {
        Self {
            ip,
            status: TaskStatus::PreProcessing,
            model_filename,
            inference_filename,
            inference_type,
            processed: 0_usize,
            unprocessed: 0_usize,
        }
    }
}
