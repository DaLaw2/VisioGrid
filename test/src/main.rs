use actix_rt::System;
use library::web::page::setting;
use library::web::page::inference;
use tokio::time::{sleep, Duration};
use actix_web::{App, Error, HttpServer};
use library::manager::task::file_manager::FileManager;

#[actix_web::main]
async fn main() -> Result<(), Error> {
    FileManager::initialize().await;
    FileManager::run().await;
    HttpServer::new(|| {
        App::new()
            .service(setting::initialize())
            .service(inference::initialize())
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await?;
    FileManager::cleanup().await;
    Ok(())
}
