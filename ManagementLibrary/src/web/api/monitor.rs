use actix_web_actors::ws::start;
use actix_web::{HttpRequest, Scope, web};
use actix_web::{get, HttpResponse, Error};
use crate::web::utils::websocket::WebSocket;

pub fn initialize() -> Scope {
    web::scope("/monitor")
        .service(get_information)
        .service(get_performance)
}

#[get("/get/information/{path}")]
async fn get_information(req: HttpRequest, path: web::Path<String>) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().body("Hello"))
}

#[get("/get/performance/{path}")]
async fn get_performance(req: HttpRequest, path: web::Path<String>) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().body("Hello"))
}

#[get("/websocket/performance/{path}")]
async fn connect(req: HttpRequest, stream: web::Payload, path: web::Path<String>) -> Result<HttpResponse, Error> {
    let websocket = WebSocket {
        interval: None,
    };
    start(websocket, &req, stream)
}
