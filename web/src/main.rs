use actix_web::{App, HttpServer};
use library::web::page::setting::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(get_config)
            .service(update_config)
            .service(setting)
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
