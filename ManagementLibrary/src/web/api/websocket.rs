use actix_web_actors::ws::start;
use actix_web::{HttpRequest, Scope, web};
use actix_web::{get, HttpResponse, Error};
use crate::web::utils::websocket::WebSocket;

pub fn initialize() -> Scope {
    web::scope("/websocket")
}

#[get("connect")]
async fn connect(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    start(WebSocket {}, &req, stream)
}
