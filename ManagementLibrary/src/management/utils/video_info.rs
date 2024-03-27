use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoInfo {
    pub format: String,
    pub bitrate: u32,
    pub framerate: String,
}

impl Default for VideoInfo {
    fn default() -> Self {
        Self {
            format: "video/x-h264".to_string(),
            bitrate: 50000_u32,
            framerate: "30/1".to_string(),
        }
    }
}
