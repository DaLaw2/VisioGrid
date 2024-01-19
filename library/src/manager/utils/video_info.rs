use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoInfo {
    pub format: String,
    pub stream_format: String,
    pub alignment: String,
    pub level: String,
    pub profile: String,
    pub width: i32,
    pub height: i32,
    pub framerate: String,
    pub pixel_aspect_ratio: String,
    pub coded_picture_structure: String,
    pub chroma_format: String,
    pub bit_depth_luma: u32,
    pub bit_depth_chroma: u32,
    pub colorimetry: String,
}

impl VideoInfo {
    pub fn default() -> Self {
        VideoInfo {
            format: String::new(),
            stream_format: String::new(),
            alignment: String::new(),
            level: String::new(),
            profile: String::new(),
            width: 0,
            height: 0,
            framerate: String::new(),
            pixel_aspect_ratio: String::new(),
            coded_picture_structure: String::new(),
            chroma_format: String::new(),
            bit_depth_luma: 0,
            bit_depth_chroma: 0,
            colorimetry: String::new(),
        }
    }
}