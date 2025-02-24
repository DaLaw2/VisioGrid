use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct FileHeader {
    pub file_name: String,
    pub packet_count: usize,
}

impl FileHeader {
    pub fn new(file_name: String, packet_count: usize) -> Self {
        Self {
            file_name,
            packet_count,
        }
    }
}
