use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoInfo {
    pub format: String,
    pub width: i32,
    pub height: i32,
    pub bitrate: u32,
    pub framerate: String,
}

impl Default for VideoInfo {
    fn default() -> Self {
        Self {
            format: "video/x-h264".to_string(),
            width: 0,
            height: 0,
            bitrate: 0,
            framerate: "30/1".to_string(),
        }
    }
}
