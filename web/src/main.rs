use actix_web::{App, HttpServer};
use library::web::page::inference;
use library::web::page::setting;
use library::manager::web::file_manager::FileManager;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    FileManager::initialize().await;
    HttpServer::new(|| {
        App::new()
            .service(setting::initialize())
            .service(inference::initialize())
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
