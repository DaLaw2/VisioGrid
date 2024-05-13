use actix_web::{get, web, HttpResponse, Scope, Responder};
use crate::utils::static_files::StaticFiles;

pub fn initialize() -> Scope {
    web::scope("/javascript")
        .service(javascript)
}

#[get("/{filename:.*\\.js}")]
async fn javascript(filename: web::Path<String>) -> impl Responder {
    let filename = filename.into_inner();
    let path = format!("javascript/{}", filename);
    match StaticFiles::get(&path) {
        Some(file) => HttpResponse::Ok().content_type("application/javascript").body(file.data),
        None => HttpResponse::NotFound().body("Not Found"),
    }
}
