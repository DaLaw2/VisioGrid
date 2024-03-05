use std::fmt::Display;
use serde::{Serialize, Deserialize};
use crate::management::utils::format::{format_ram, format_vram};

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentInformation {
    pub host_name: String,
    pub system_name: String,
    pub cpu: String,
    pub cores: usize,
    pub ram: u64,
    pub gpu: String,
    pub vram: u64,
}

impl Display for AgentInformation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = format!("Host Name: {}, System Name: {}, CPU Model: {}, Cores: {}, RAM Total: {}, GPU Model: {}, VRAM Total: {}",
            self.host_name, self.system_name, self.cpu, self.cores, format_ram(self.ram), self.gpu, format_vram(self.vram)
        );
        write!(f, "{}", str)
    }
}
