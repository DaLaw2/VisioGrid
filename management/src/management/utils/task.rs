use serde::Serialize;
use crate::management::utils::inference_argument::InferenceArgument;
use crate::management::utils::inference_task::InferenceTask;
use uuid::Uuid;

#[derive(Serialize, Debug, Copy, Clone)]
pub enum TaskStatus {
    Waiting,
    PreProcessing,
    Processing,
    PostProcessing,
    Success,
    Fail,
}

#[derive(Serialize, Debug, Clone)]
pub struct Task {
    pub uuid: Uuid,
    pub status: TaskStatus,
    pub failed: usize,
    pub success: usize,
    pub unprocessed: usize,
    pub model_file_name: String,
    pub media_file_name: String,
    pub inference_argument: InferenceArgument,
    pub result: Vec<InferenceTask>,
    pub error: Result<(), String>,
}

impl Task {
    pub async fn new(uuid: Uuid, model_file_name: String, media_file_name: String, inference_argument: InferenceArgument) -> Self {
        Self {
            uuid,
            status: TaskStatus::Waiting,
            failed: 0_usize,
            success: 0_usize,
            unprocessed: 0_usize,
            model_file_name,
            media_file_name,
            inference_argument,
            result: Vec::new(),
            error: Ok(()),
        }
    }
}
