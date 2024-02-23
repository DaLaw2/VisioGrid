#![allow(non_snake_case)]

use ManagementLibrary::management::manager::Manager;

#[actix_web::main]
async fn main() {
    Manager::run().await;
    Manager::terminate().await;
}
