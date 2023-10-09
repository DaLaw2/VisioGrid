pub enum InferenceType {
    YOLO,
    PyTorch,
    TensorFlow,
    ONNX,
    Default
}

pub enum TaskStatus {
    PreProcessing,
    Processing,
    PostProcessing,
    Fail
}

pub struct Task {
    pub ip: String,
    pub status: TaskStatus,
    pub model_filename: String,
    pub inference_filename: String,
    pub inference_type: InferenceType,
    pub processed: usize,
}

impl Task {
    pub fn new(ip: String, model_filename: String, inference_filename: String, inference_type: InferenceType) -> Task {
        Self {
            ip,
            status: TaskStatus::PreProcessing,
            model_filename,
            inference_filename,
            inference_type,
            processed: 0_usize,
        }
    }
}