use actix_web::{App, HttpServer};
use library::web::page::setting;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(setting::initialize())
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
