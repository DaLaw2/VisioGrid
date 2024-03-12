use actix_web::{get, web, HttpResponse, Scope, Responder};
use crate::utils::static_files::StaticFiles;

pub fn initialize() -> Scope {
    web::scope("/misc")
        .service(misc)
}

#[get("/{filename}")]
async fn misc(info: web::Path<(String,)>) -> impl Responder {
    let filename = info.into_inner().0;
    let path = format!("misc/{}", filename);
    match StaticFiles::get(&path) {
        Some(file) => HttpResponse::Ok().body(file.data),
        None => HttpResponse::NotFound().body("Not Found"),
    }
}
