use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use actix_multipart::Multipart;
use std::path::{Path, PathBuf};
use sanitize_filename::sanitize;
use futures::{self, StreamExt, TryStreamExt};
use actix_web::{get, post, web, Scope, Result, Error, HttpResponse, Responder};
use crate::utils::static_files::StaticFiles;

pub fn initialize() -> Scope {
    web::scope("/inference")
        .service(inference)
        .service(save_model)
        .service(save_files)
}

#[get("")]
pub async fn inference() -> impl Responder {
    let html = StaticFiles::get("inference.html").expect("File not found in static files.").data;
    let response = HttpResponse::Ok().content_type("text/html").body(html);
    response
}

#[post("/save_model")]
async fn save_model(mut payload: Multipart) -> Result<HttpResponse, Error> {
    while let Ok(Some(mut field)) = payload.try_next().await {
        let file_name = match field.content_disposition().get_filename() {
            Some(name) => name,
            None => continue
        };
        let file_name = sanitize(file_name);
        if file_name.is_empty() {
            return Err(actix_web::error::ErrorBadRequest("Invalid filename."));
        }
        let extension = Path::new(&file_name).extension().and_then(|os_str| os_str.to_str()).unwrap_or("");
        if !(match extension {
            "pt" | "pth" | "h5" | "cfg" | "weights" | "names" | "onnx" => true,
            _ => false
        }) {
            return Err(actix_web::error::ErrorBadRequest("Invalid file extension."));
        }
        let mut file_path = PathBuf::from("./SavedModel");
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

#[post("/save_file")]
async fn save_files(mut payload: Multipart) -> Result<HttpResponse, Error> {
    while let Ok(Some(mut field)) = payload.try_next().await {
        let file_name = match field.content_disposition().get_filename() {
            Some(name) => name,
            None => continue
        };
        let file_name = sanitize(file_name);
        if file_name.is_empty() {
            return Err(actix_web::error::ErrorBadRequest("Invalid filename."));
        }
        let extension = Path::new(&file_name).extension().and_then(|os_str| os_str.to_str()).unwrap_or("");
        if !(match extension {
            "jpg" | "jpeg" | "gif" | "mp4" | "wav" | "avi" | "mkv" | "zip" => true,
            _ => false
        }) {
            return Err(actix_web::error::ErrorBadRequest("Invalid file extension."));
        }
        let mut file_path = PathBuf::from("./SavedFile");
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
