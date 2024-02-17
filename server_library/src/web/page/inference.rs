use uuid::Uuid;
use tokio::fs::File;
use std::path::Path;
use std::str::FromStr;
use tokio::io::AsyncWriteExt;
use actix_multipart::Multipart;
use sanitize_filename::sanitize;
use futures::{self, StreamExt, TryStreamExt};
use actix_web::{get, post, web, Scope, HttpResponse, Responder};
use crate::manager::utils::task::Task;
use crate::utils::static_files::StaticFiles;
use crate::manager::file_manager::FileManager;
use crate::web::utils::response::OperationStatus;
use crate::manager::utils::inference_type::InferenceType;

pub fn initialize() -> Scope {
    web::scope("/inference")
        .service(page)
        .service(save_files)
}

#[get("")]
async fn page() -> impl Responder {
    let html = StaticFiles::get("inference.html").expect("File not found in static files.").data;
    HttpResponse::Ok().content_type("text/html").body(html)
}

#[post("/save_file")]
async fn save_files(mut payload: Multipart) -> impl Responder {
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
                    Err(_) => return HttpResponse::InternalServerError().json(web::Json(OperationStatus::new(false, None)))
                };
                model_type = String::from_utf8_lossy(&content).to_string();
            }
            continue;
        }
        let (field_name, mut file_name) = match (content_disposition.get_name(), content_disposition.get_filename()) {
            (Some(field_name), Some(file_name)) => (field_name, sanitize(file_name)),
            _ => continue,
        };
        if file_name.is_empty() {
            return HttpResponse::BadRequest().json(web::Json(OperationStatus::new(false, Some("Invalid filename.".to_string()))));
        }
        file_name = format!("{}_{}_{}", uuid, model_type, file_name);
        let file_extension = Path::new(&file_name).extension().and_then(|os_str| os_str.to_str()).unwrap_or("");
        let file_path = match (field_name, file_extension) {
            ("ptFile" | "onnxFile", "pt" | "onnx") => {
                model_filename = file_name.clone();
                Path::new(".").join("SavedModel").join(file_name)
            },
            ("yoloInferenceFile" | "onnxInferenceFile" | "defaultInferenceFile", "png" | "jpg" | "jpeg" | "mp4" | "avi" | "mkv" | "zip") => {
                media_filename = file_name.clone();
                Path::new(".").join("SavedFile").join(file_name)
            },
            _ => return HttpResponse::BadRequest().json(web::Json(OperationStatus::new(false, Some("Invalid file type or extension.".to_string())))),
        };
        match File::create(&file_path).await {
            Ok(mut file) => {
                while let Some(chunk) = field.next().await {
                    match chunk {
                        Ok(data) => {
                            if let Err(_) = file.write_all(&data).await {
                                return HttpResponse::InternalServerError().json(web::Json(OperationStatus::new(false, None)))
                            }
                        },
                        Err(_) => return HttpResponse::InternalServerError().json(web::Json(OperationStatus::new(false, None))),
                    }
                }
            }
            Err(_) => return HttpResponse::InternalServerError().json(web::Json(OperationStatus::new(false, None))),
        }
    }
    match InferenceType::from_str(&*model_type) {
        Ok(inference_type) => {
            let new_task = Task::new(uuid, model_filename, media_filename, inference_type).await;
            FileManager::add_pre_process_task(new_task).await;
            HttpResponse::Ok().json(OperationStatus::new(true, None))
        },
        Err(_) => HttpResponse::BadRequest().json(web::Json(OperationStatus::new(false, Some("Invalid inference type.".to_string())))),
    }
}