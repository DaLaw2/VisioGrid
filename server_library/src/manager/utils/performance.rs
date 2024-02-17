use serde::{Serialize, Deserialize};
use crate::manager::utils::node_information::NodeInformation;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Performance {
    pub cpu: f64,
    pub ram: f64,
    pub gpu: f64,
    pub vram: f64,
}

impl Performance {
    pub fn default() -> Self {
        Self {
            cpu: 0.0,
            ram: 0.0,
            gpu: 0.0,
            vram: 0.0,
        }
    }

    pub fn new(cpu: f64, ram: f64, gpu: f64, vram: f64) -> Self {
        Self {
            cpu,
            ram,
            gpu,
            vram,
        }
    }

    pub fn calc_residual_usage(node_information: &NodeInformation, realtime_performance: &Performance) -> Performance {
        Self {
            cpu: 100_f64 - realtime_performance.cpu,
            ram: node_information.ram as f64 - realtime_performance.ram,
            gpu: 100_f64 - realtime_performance.gpu,
            vram: node_information.vram as f64 - realtime_performance.vram,
        }
    }
}