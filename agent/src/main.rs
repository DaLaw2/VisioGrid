use crate::management::management::Management;

pub mod connection;
pub mod management;
pub mod utils;

#[tokio::main]
async fn main() {
    Management::run().await;
    Management::terminate().await;
}
