use uuid::Uuid;
use crate::management::utils::image_task::ImageTask;
use crate::management::result_repository::ResultRepository;
use crate::management::utils::inference_type::InferenceType;

#[derive(Debug, Copy, Clone)]
pub enum TaskStatus {
    PreProcessing,
    Waiting,
    Processing,
    PostProcessing,
    Fail,
    Success,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub uuid: Uuid,
    pub status: TaskStatus,
    pub failed: usize,
    pub success: usize,
    pub unprocessed: usize,
    pub error: Result<(), String>,
    pub model_filename: String,
    pub media_filename: String,
    pub inference_type: InferenceType,
    pub result: Vec<ImageTask>,
}

impl Task {
    pub async fn new(uuid: Uuid, model_filename: String, media_filename: String, inference_type: InferenceType) -> Self {
        Self {
            uuid,
            status: TaskStatus::Waiting,
            failed: 0_usize,
            success: 0_usize,
            unprocessed: 0_usize,
            error: Ok(()),
            model_filename,
            media_filename,
            inference_type,
            result: Vec::new(),
        }
    }

    pub fn change_status(&mut self, status: TaskStatus) {
        self.status = status;
    }

    pub async fn panic(mut self, error_message: String) {
        self.status = TaskStatus::Fail;
        self.error = Err(error_message);
        ResultRepository::task_failed(self).await;
    }

    pub async fn update_unprocessed(&mut self, unprocessed: usize) {
        self.unprocessed = unprocessed;
    }
}
