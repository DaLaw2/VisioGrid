use uuid::Uuid;
use tokio::fs::File;
use std::path::Path;
use std::str::FromStr;
use tokio::io::AsyncWriteExt;
use actix_multipart::Multipart;
use sanitize_filename::sanitize;
use futures::{self, StreamExt, TryStreamExt};
use actix_web::{get, post, web, Scope, Result, Error, HttpResponse, Responder};
use crate::manager::utils::task::Task;
use crate::utils::static_files::StaticFiles;
use crate::manager::file_manager::FileManager;
use crate::web::utils::response::OperationStatus;
use crate::manager::utils::inference_type::InferenceType;

pub fn initialize() -> Scope {
    web::scope("/inference")
        .service(inference)
        .service(save_files)
}

#[get("")]
async fn inference() -> impl Responder {
    let html = StaticFiles::get("inference.html").expect("File not found in static files.").data;
    HttpResponse::Ok().content_type("text/html").body(html)
}

#[post("/save_file")]
async fn save_files(mut payload: Multipart) -> Result<HttpResponse, Error> {
    let uuid = Uuid::new_v4();
    let mut model_type = String::new();
    let mut model_filename = String::new();
    let mut media_filename = String::new();
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        if model_type.is_empty() {
            if let Some(chunk) = field.next().await {
                let content = match chunk {
                    Ok(data) => data,
                    Err(err) => return Ok(HttpResponse::InternalServerError().json(web::Json(OperationStatus::new(false, Some(format!("{}", err))))))
                };
                model_type = String::from_utf8_lossy(&content).to_string();
            }
            continue;
        }
        let (field_name, mut file_name) = match (content_disposition.get_name(), content_disposition.get_filename()) {
            (Some(field_name), Some(file_name)) => (field_name, sanitize(file_name)),
            _ => continue
        };
        if file_name.is_empty() {
            return Ok(HttpResponse::BadRequest().json(web::Json(OperationStatus::new(false, Some("Invalid filename.".to_string())))));
        }
        file_name = format!("{}_{}_{}", uuid, model_type, file_name);
        let file_extension = Path::new(&file_name).extension().and_then(|os_str| os_str.to_str()).unwrap_or("");
        let file_path = match (field_name, file_extension) {
            ("ptFile" | "onnxFile", "pt" | "onnx") => {
                model_filename = file_name.clone();
                Path::new(".").join("SavedModel").join(file_name)
            },
            ("yoloInferenceFile" | "onnxInferenceFile" | "defaultInferenceFile", "png" | "jpg" | "jpeg" | "gif" | "mp4" | "wav" | "avi" | "mkv" | "zip") => {
                media_filename = file_name.clone();
                Path::new(".").join("SavedFile").join(file_name)
            },
            _ => return Ok(HttpResponse::BadRequest().json(web::Json(OperationStatus::new(false, Some("Invalid file type or extension.".to_string())))))
        };
        let mut file = File::create(&file_path).await?;
        while let Some(chunk) = field.next().await {
            match chunk {
                Ok(data) => file.write_all(&data).await?,
                Err(err) => return Ok(HttpResponse::InternalServerError().json(web::Json(OperationStatus::new(false, Some(format!("{}", err))))))
            }
        }
    }
    //Make sure no errors occur
    let new_task = Task::new(uuid, model_filename, media_filename, InferenceType::from_str(&*model_type).unwrap()).await;
    FileManager::add_preprocess_task(new_task).await;
    Ok(HttpResponse::Ok().json(OperationStatus::new(true, None)))
}
