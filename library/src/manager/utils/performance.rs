pub struct Performance {
    pub cpu: f64,
    pub ram: f64,
    pub gpu: f64,
    pub vram: f64,
}

impl Performance {
    pub fn new(cpu: f64, ram: f64, gpu: f64, vram: f64) -> Performance {
        Performance {
            cpu,
            ram,
            gpu,
            vram,
        }
    }
}
