use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct FileHeader {
    filename: String,
    filesize: usize,
    packet_count: usize,
}

impl FileHeader {
    pub fn new(filename: String, filesize: usize) -> Self {
        let packet_count = (filesize + 1048575_usize) / 1048576_usize;
        Self {
            filename,
            filesize,
            packet_count,
        }
    }
}