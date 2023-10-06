use tokio::fs::File;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use actix_multipart::Multipart;
use sanitize_filename::sanitize;
use futures::{self, StreamExt, TryStreamExt};
use actix_web::{post, web, Error, HttpResponse, Result, get, Scope, Responder};

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
        if file_name.is_empty() {
            return Err(actix_web::error::ErrorBadRequest("Invalid filename"));
        }
        let mut file_path = PathBuf::from("./WebSave");
        file_path.push(file_name);
        let mut f = File::create(&file_path).await?;
        while let Some(chunk) = field.next().await {
            match chunk {
                Ok(data) => f.write_all(&data).await?,
                Err(e) => return Err(actix_web::error::ErrorInternalServerError(e)),
            }
        }
    }
    Ok(HttpResponse::Ok().into())
}
