use uuid::Uuid;
use crate::manager::utils::inference_type::InferenceType;

#[derive(Copy, Clone)]
pub enum TaskStatus {
    PreProcessing,
    Waiting,
    Processing,
    PostProcessing,
    Fail,
}

#[derive(Clone)]
pub struct Task {
    pub uuid: Uuid,
    pub status: TaskStatus,
    pub processed: usize,
    pub unprocessed: usize,
    pub ip: String,
    pub model_filename: String,
    pub image_filename: String,
    pub inference_type: InferenceType,
}

impl Task {
    pub async fn new(ip: String, model_filename: String, image_filename: String, inference_type: InferenceType) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            status: TaskStatus::PreProcessing,
            processed: 0_usize,
            unprocessed: 0_usize,
            ip,
            model_filename,
            image_filename,
            inference_type,
        }
    }
}
