use std::path::PathBuf;
use actix_web::{post, web, Error, HttpResponse, Result, get, Scope, Responder};
use actix_multipart::Multipart;
use futures::{StreamExt, TryStreamExt};
use sanitize_filename::sanitize;
use std::io::{self, Write};

pub fn initialize() -> Scope {
    web::scope("/inference")
        .service(inference)
        .service(upload)
}

#[get("")]
pub async fn inference() -> impl Responder {
    web::Json("test")
}
#[post("/upload")]
async fn upload(mut payload: Multipart) -> Result<HttpResponse, Error> {
    while let Ok(Some(mut field)) = payload.try_next().await {
        let file_name = match field.content_disposition().get_name() {
            Some(name) => name,
            None => continue
        };
        let file_name = sanitize(file_name);
        let mut file_path = PathBuf::from("./WebSave");
        file_path.push(file_name);
        let mut f = web::block(|| std::fs::File::create(&file_path)).await?;
        while let Some(chunk) = field.next().await {
            let data = chunk?;
            let result = web::block(move || {
                match f {
                    Ok(ref mut file) => match file.write_all(&data) {
                        Ok(_) => Ok(f),
                        Err(e) => Err(e),
                    },
                    Err(e) => Err(e),
                }
            }).await;
            match result {
                Ok(file) => f = file,
                Err(err) => return Err(err.into()),
            }
        }
    }
    Ok(HttpResponse::Ok().into())
}
