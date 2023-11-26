use serde::{Serialize, Deserialize};

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
}
