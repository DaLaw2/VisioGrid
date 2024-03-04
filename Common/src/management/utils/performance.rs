use serde::{Serialize, Deserialize};
use crate::management::utils::agent_information::AgentInformation;

#[derive(Serialize, Deserialize, Clone)]
pub struct Performance {
    pub cpu: u64,
    pub ram: u64,
    pub gpu: u64,
    pub vram: u64,
}

impl Performance {
    pub fn default() -> Self {
        Self {
            cpu: 0,
            ram: 0,
            gpu: 0,
            vram: 0,
        }
    }

    pub fn new(cpu: u64, ram: u64, gpu: u64, vram: u64) -> Self {
        Self {
            cpu,
            ram,
            gpu,
            vram,
        }
    }

    pub fn calc_residual_usage(agent_information: &AgentInformation, realtime_performance: &Performance) -> Performance {
        Self {
            cpu: 100_u64 - realtime_performance.cpu,
            ram: agent_information.ram - realtime_performance.ram,
            gpu: 100_u64 - realtime_performance.gpu,
            vram: agent_information.vram - realtime_performance.vram,
        }
    }
}
