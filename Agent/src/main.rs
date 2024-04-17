#![allow(non_snake_case)]

use AgentLibrary::management::monitor::Monitor;

#[tokio::main]
async fn main() {
    Monitor::run().await;
    loop {}
}
