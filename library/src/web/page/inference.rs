use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use actix_multipart::Multipart;
use std::path::{Path, PathBuf};
use sanitize_filename::sanitize;
use futures::{self, StreamExt, TryStreamExt};
use actix_web::{get, post, web, Scope, Result, Error, HttpResponse, Responder};
use crate::utils::static_files::StaticFiles;
use crate::web::utils::response::OperationStatus;

pub fn initialize() -> Scope {
    web::scope("/inference")
        .service(inference)
        .service(save_files)
}

#[get("")]
pub async fn inference() -> impl Responder {
    let html = StaticFiles::get("inference.html").expect("File not found in static files.").data;
    let response = HttpResponse::Ok().content_type("text/html").body(html);
    response
}

#[post("/save_file")]
async fn save_files(mut payload: Multipart) -> Result<HttpResponse, Error> {
    while let Ok(Some(mut field)) = payload.try_next().await {
        let field_name = match field.content_disposition().get_name() {
            Some(name) => name,
            None => continue
        };
        let file_name = match field.content_disposition().get_filename() {
            Some(name) => name,
            None => continue
        };
        let file_name = sanitize(file_name);
        if file_name.is_empty() {
            return Ok(HttpResponse::BadRequest().json(web::Json(OperationStatus::new(false, Some("Invalid filename.".to_string())))));
        }
        let file_extension = Path::new(&file_name).extension().and_then(|os_str| os_str.to_str()).unwrap_or("");
        let file_path = match (field_name, file_extension) {
            ("ptFile" | "h5File" | "cfgFile" | "weightsFile" | "namesFile" | "onnxFile", "pt" | "pth" | "h5" | "cfg" | "weights" | "names" | "onnx") => "./SavedModel",
            ("yoloInferenceFile" | "pytorchInferenceFile" | "tensorflowInferenceFile" | "onnxInferenceFile" | "defaultInferenceFile", "jpg" | "jpeg" | "gif" | "mp4" | "wav" | "avi" | "mkv" | "zip") => "./SavedFile",
            _ => return Ok(HttpResponse::BadRequest().json(web::Json(OperationStatus::new(false, Some("Invalid file type or extension.".to_string())))))
        };
        let mut file_path = PathBuf::from(file_path);
        file_path.push(file_name);
        let mut f = File::create(&file_path).await?;
        while let Some(chunk) = field.next().await {
            match chunk {
                Ok(data) => f.write_all(&data).await?,
                Err(e) => return Ok(HttpResponse::InternalServerError().json(web::Json(OperationStatus::new(false, Some(format!("{}", e))))))
            }
        }
    }
    Ok(HttpResponse::Ok().json(OperationStatus::new(true, None)))
}
