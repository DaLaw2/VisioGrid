#![allow(non_snake_case)]

use library::manager::server::Server;

#[actix_web::main]
async fn main() {
    Server::run().await;
    Server::terminate().await;
}
