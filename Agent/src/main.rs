#![allow(non_snake_case)]

use tokio::time::sleep;
use std::time::Duration;
use AgentLibrary::management::monitor::Monitor;

pub fn cuda_is_available(){
    println!("Cuda available: {}", tch::Cuda::is_available());
    println!("Cudnn available: {}", tch::Cuda::cudnn_is_available());
    let device = tch::Device::cuda_if_available();
    println!("Device :{:?}",device);
}

#[tokio::main]
async fn main() {
    cuda_is_available();
    Monitor::run().await;
    loop {}
}
