use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct FileTransferResult {
    pub result: Option<Vec<usize>>,
}

impl FileTransferResult {
    pub fn new(result: Option<Vec<usize>>) -> Self {
        Self {
            result,
        }
    }

    pub fn into(self) -> Option<Vec<usize>> {
        self.result
    }
}
