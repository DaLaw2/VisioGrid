use crate::utils::config::Config;
use actix_web::{get, post, web, HttpResponse, Responder, Scope};

pub fn initialize() -> Scope {
    web::scope("/config")
        .service(get_config)
        .service(update_config)
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
