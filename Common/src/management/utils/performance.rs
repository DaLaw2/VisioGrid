use std::fmt::Display;
use serde::{Serialize, Deserialize};
use crate::management::utils::format::format_bytes;
use crate::management::utils::agent_information::AgentInformation;

#[derive(Serialize, Deserialize, Clone)]
pub struct Performance {
    pub cpu: f64,
    pub ram: f64,
    pub gpu: f64,
    pub vram: f64,
}

impl Performance {
    pub fn new(cpu: f64, ram: f64, gpu: f64, vram: f64) -> Self {
        Self {
            cpu,
            ram,
            gpu,
            vram,
        }
    }

    pub fn default() -> Self {
        Self {
            cpu: 0.0,
            ram: 0.0,
            gpu: 0.0,
            vram: 0.0,
        }
    }

    pub fn calc_residual_usage(agent_information: &AgentInformation, realtime_performance: &Performance) -> Performance {
        Self {
            cpu: 100_f64 - realtime_performance.cpu,
            ram: agent_information.ram - realtime_performance.ram,
            gpu: 100_f64 - realtime_performance.gpu,
            vram: agent_information.vram - realtime_performance.vram,
        }
    }
}

impl Display for Performance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ram_format = format_bytes(self.ram);
        let vram_format = format_bytes(self.vram);
        let str = format!("CPU: {:.2}％, RAM: {}, GPU: {}％, VRAM: {}",
            self.cpu, ram_format, self.gpu, vram_format
        );
        write!(f, "{}", str)
    }
}
