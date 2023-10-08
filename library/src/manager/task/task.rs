use crate::manager::task::inference_type::InferenceType;

pub enum TaskStatus {
    PreProcessing,
    Processing,
    PostProcessing
}

pub struct Task {
    status: TaskStatus,
    inference_type: InferenceType,
    fail_times: usize,
}