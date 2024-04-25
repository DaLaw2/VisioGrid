use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct FileHeader {
    pub filename: String,
    pub packet_count: usize,
}

impl FileHeader {
    pub fn new(filename: String, packet_count: usize) -> Self {
        Self {
            filename,
            packet_count,
        }
    }
}
