#![allow(non_snake_case)]

use tch::Tensor;
use tokio::process::Command;
use tokio::time::Instant;
use AgentLibrary::management::manager::Manager;

pub fn demo() {
    let mut t = Tensor::from_slice(&[3, 1, 4, 1, 5]);
    t.print();
    t = t * 2;
    t.print()
}

pub fn cuda_is_available(){
    println!("Cuda available: {}", tch::Cuda::is_available());
    println!("Cudnn available: {}", tch::Cuda::cudnn_is_available());
    let device = tch::Device::cuda_if_available();
    println!("Device :{:?}",device);
    let t = Tensor::from_slice(&[1,2,3,4,5]).to(device);
    t.print();
}

#[tokio::main]
async fn main() {
    demo();
    let now = Instant::now();
    cuda_is_available();
    let elapsed = now.elapsed();

    let _ = Command::new("nvidia-smi")
        .output()
        .await
        .expect("Failed to execute command");


    println!("Command took: {:.2?}", elapsed);


    Manager::run().await;
    Manager::terminate().await;
    panic!("Monitor: Fail to get gpu information.")
}
