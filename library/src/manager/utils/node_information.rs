use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeInformation {
    device_name: String,
    os: String,
    cpu: String,
    cores: usize,
    ram: usize,
    gpu: String,
    gram: usize,
}