#![allow(non_snake_case)]

use ClientLibrary::manager::client::Client;

#[actix_web::main]
async fn main() {
    Client::run().await;
    Client::terminate().await;
}
