use library::web::page::inference;
use library::web::page::javascript;
use library::web::page::configuration;
use actix_web::{App, Error, HttpServer};
use library::manager::file_manager::FileManager;

#[actix_web::main]
async fn main() -> Result<(), Error> {
    FileManager::initialize().await;
    FileManager::run().await;
    HttpServer::new(|| {
        App::new()
            .service(configuration::initialize())
            .service(inference::initialize())
            .service(javascript::initialize())
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await?;
    FileManager::cleanup().await;
    Ok(())
}
