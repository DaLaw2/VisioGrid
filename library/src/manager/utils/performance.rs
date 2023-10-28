pub struct Performance {
    cpu: f64,
    ram: f64,
    gpu: f64,
    vram: f64,
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
