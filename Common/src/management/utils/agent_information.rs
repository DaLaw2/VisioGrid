use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentInformation {
    pub host_name: String,
    pub os: String,
    pub cpu: String,
    pub cores: usize,
    pub ram: usize,
    pub gpu: String,
    pub vram: usize,
}
