use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct TaskResult {
    pub status: Result<(), String>,
}

impl TaskResult {
    pub fn new(result: Result<(), String>) -> Self {
        Self {
            status: result,
        }
    }

    pub fn into(self) -> Result<(), String> {
        self.status
    }
}
