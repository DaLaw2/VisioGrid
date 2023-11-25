use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct NodeInformation {
    device_name: String,
    os: String,
    cpu: String,
    cores: usize,
    ram: usize,
    gpu: String,
    gram:usize,
}
