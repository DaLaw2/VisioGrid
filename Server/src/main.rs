#![allow(non_snake_case)]

use ServerLibrary::manager::server::Server;

#[actix_web::main]
async fn main() {
    Server::run().await;
    Server::terminate().await;
}
