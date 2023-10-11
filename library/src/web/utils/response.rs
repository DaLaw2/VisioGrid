use serde::Serialize;

#[derive(Serialize)]
pub struct OperationStatus {
    success: bool,
    error: Option<String>,
}

impl OperationStatus {
    pub fn new(success: bool, error: Option<String>) -> Self {
        Self {
            success,
            error
        }
    }
}