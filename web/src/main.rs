use std::time::Duration;
use actix_web::{App, HttpServer};
use library::web::page::inference;
use library::web::page::setting;
use library::manager::task::file_manager::FileManager;

#[actix_web::main]
async fn main() {
    FileManager::initialize().await;
    FileManager::run().await;
    HttpServer::new(|| {
        App::new()
            .service(setting::initialize())
            .service(inference::initialize())
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await.unwrap();
    tokio::time::sleep(Duration::from_secs(60)).await;
    FileManager::cleanup()
}
