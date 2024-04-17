#![allow(non_snake_case)]

use ManagementLibrary::management::management::Management;

#[actix_web::main]
async fn main() {
    Management::run().await;
    Management::terminate().await;
}
