use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct NodeInformation {
    pub device_name: String,
    pub os: String,
    pub cpu: String,
    pub cores: usize,
    pub ram: usize,
    pub gpu: String,
    pub vram: usize,
}
