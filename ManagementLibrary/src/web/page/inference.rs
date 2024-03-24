use uuid::Uuid;
use tokio::fs::File;
use std::path::Path;
use std::str::FromStr;
use tokio::io::AsyncWriteExt;
use actix_multipart::Multipart;
use sanitize_filename::sanitize;
use futures::{self, StreamExt, TryStreamExt};
use actix_web::{get, post, web, Scope, HttpResponse, Responder};
use crate::management::utils::task::Task;
use crate::utils::static_files::StaticFiles;
use crate::management::file_manager::FileManager;
use crate::web::utils::response::OperationStatus;
use crate::management::utils::model_type::ModelType;

pub fn initialize() -> Scope {
    web::scope("/inference")
        .service(page)
        .service(save_files)
}

#[get("")]
async fn page() -> impl Responder {
    let html = StaticFiles::get("html/inference.html").expect("File not found in static files.").data;
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
            _ => return HttpResponse::BadRequest().json(web::Json(OperationStatus::new(false, Some("Invalid payload.".to_string())))),
        };
        if file_name.is_empty() {
            return HttpResponse::BadRequest().json(web::Json(OperationStatus::new(false, Some("Invalid filename.".to_string()))));
        }
        file_name = format!("{}_{}", uuid, file_name);
        let file_extension = Path::new(&file_name).extension().and_then(|os_str| os_str.to_str()).unwrap_or("");
        let file_path = match (field_name, file_extension) {
            ("modelFile", "pt" | "pth" | "onnx") => {
                model_filename = file_name.clone();
                Path::new(".").join("SavedModel").join(file_name)
            },
            ("inferenceFile", "png" | "jpg" | "jpeg" | "mp4" | "avi" | "mkv" | "zip") => {
                media_filename = file_name.clone();
                Path::new(".").join("SavedFile").join(file_name)
            },
            _ => return HttpResponse::BadRequest().json(web::Json(OperationStatus::new(false, Some("Invalid file type or extension.".to_string())))),
        };
        if let Ok(mut file) = File::create(&file_path).await {
            while let Some(chunk) = field.next().await {
                if let Ok(data) = chunk {
                    if file.write_all(&data).await.is_err() {
                        return HttpResponse::InternalServerError().json(web::Json(OperationStatus::new(false, None)))
                    }
                } else {
                    return HttpResponse::InternalServerError().json(web::Json(OperationStatus::new(false, None)))
                }
            }
        } else {
            return HttpResponse::InternalServerError().json(web::Json(OperationStatus::new(false, None)))
        }
    }
    if let Ok(model_type) = ModelType::from_str(&*model_type) {
        let new_task = Task::new(uuid, model_filename, media_filename, model_type).await;
        FileManager::add_pre_process_task(new_task).await;
        HttpResponse::Ok().json(OperationStatus::new(true, None))
    } else {
        HttpResponse::BadRequest().json(web::Json(OperationStatus::new(false, Some("Invalid inference type.".to_string()))))
    }
}
