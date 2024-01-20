use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoInfo {
    pub format: String,
    pub framerate: String,
}

impl Default for VideoInfo {
    fn default() -> Self {
        Self {
            format: "video/x-h264".to_string(),
            framerate: "30/1".to_string(),
        }
    }
}
