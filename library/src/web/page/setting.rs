use crate::utils::config::Config;
use crate::utils::static_files::StaticFiles;
use actix_web::{get, post, web, Responder, HttpResponse, Scope};

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
    web::Json(Config::instance().await)
}

#[post("/update_config")]
async fn update_config(config: web::Json<Config>) -> impl Responder {
    Config::update(config.into_inner()).await;
    HttpResponse::Ok().body("Configuration updated successfully.")
}
