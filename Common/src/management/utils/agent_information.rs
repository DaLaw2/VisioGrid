use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentInformation {
    pub host_name: String,
    pub system_name: String,
    pub cpu: String,
    pub cores: usize,
    pub ram: u64,
    pub gpu: String,
    pub vram: usize,
}
