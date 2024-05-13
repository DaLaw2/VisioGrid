use actix_web::{get, post, web, Responder, HttpResponse, Scope};
use crate::utils::config::Config;
use crate::utils::static_files::StaticFiles;

pub fn initialize() -> Scope {
    web::scope("/config")
        .service(page)
        .service(get_config)
        .service(update_config)
}

#[get("")]
async fn page() -> impl Responder {
    let html = StaticFiles::get("html/config.html").expect("File not found in static files.").data;
    HttpResponse::Ok().content_type("text/html").body(html)
}

#[get("/get")]
async fn get_config() -> impl Responder {
    web::Json(Config::now().await)
}

#[post("/update")]
async fn update_config(config: web::Json<Config>) -> impl Responder {
    let config = config.into_inner();
    if Config::validate(&config) {
        Config::update(config).await;
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::BadRequest().body("Invalid configuration.")
    }
}
