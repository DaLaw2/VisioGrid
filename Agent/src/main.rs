#![allow(non_snake_case)]

use AgentLibrary::management::management::Management;

#[tokio::main]
async fn main() {
    Management::run().await;
    Management::terminate().await;
}
