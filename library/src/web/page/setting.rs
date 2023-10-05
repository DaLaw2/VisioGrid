use actix_web::{get, post, web, Responder, HttpResponse, Scope};
use crate::utils::config::Config;
use crate::utils::static_files::StaticFiles;

pub fn initialize() -> Scope {
    web::scope("/setting")
        .service(setting)
        .service(get_config)
        .service(update_config)
}

#[get("")]
async fn setting() -> impl Responder {
    let html = StaticFiles::get("setting.html").expect("File not found in static files.").data;
    let response = HttpResponse::Ok().content_type("text/html").body(html);
    response
}

#[get("/get_config")]
async fn get_config() -> impl Responder {
    let config = Config::instance().await;
    web::Json(config)
}

#[post("/update_config")]
async fn update_config(config: web::Json<Config>) -> impl Responder {
    let config = config.into_inner();
    Config::update(config).await;
    HttpResponse::Ok().body("Configuration updated successfully.")
}
