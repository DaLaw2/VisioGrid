use crate::management::management::Management;

pub mod connection;
pub mod management;
pub mod utils;
pub mod web;

#[actix_web::main]
async fn main() {
    Management::run().await;
    Management::terminate().await;
}
