#![allow(non_snake_case)]

use tch::Tensor;
use ClientLibrary::manager::agent::Client;

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

#[actix_web::main]
async fn main() {
    demo();
    cuda_is_available();
    Client::run().await;
    Client::terminate().await;
}
