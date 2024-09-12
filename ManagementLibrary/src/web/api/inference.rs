use uuid::Uuid;
use tokio::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio::io::AsyncWriteExt;
use actix_multipart::{Field, Multipart};
use sanitize_filename::sanitize;
use futures::{StreamExt, TryStreamExt};
use actix_web::{get, post, web, Scope, HttpResponse, Responder};
use actix_web::http::header::ContentDisposition;
use crate::management::utils::task::Task;
use crate::utils::static_files::StaticFiles;
use crate::management::file_manager::FileManager;
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
    let mut model_type = None;
    let mut model_filename = String::new();
    let mut media_filename = String::new();
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = match field.content_disposition() {
            Some(content_disposition) => content_disposition,
            None => return HttpResponse::InternalServerError().finish(),
        };
        if let Some(field_name) = get_field_name(&content_disposition) {
            if field_name == "modelType" {
                model_type = parse_model_type(&mut field).await;
            } else {
                if let Some(mut file_name) = get_file_name(&content_disposition) {
                    let sanitized_file_name = sanitize(file_name);
                    if sanitized_file_name.is_empty() {
                        return HttpResponse::BadRequest().body("Invalid filename.");
                    }
                    file_name = format!("{}_{}", uuid, sanitized_file_name);
                    let file_extension = Path::new(&file_name).extension()
                        .and_then(|os_str| os_str.to_str()).unwrap_or("");
                    let file_path = match (&*field_name, file_extension) {
                        ("modelFile", "pt" | "pth" | "onnx") => {
                            model_filename = file_name.clone();
                            Path::new(".").join("SavedModel").join(file_name)

                        },
                        ("inferenceFile", "png" | "jpg" | "jpeg" | "mp4" | "avi" | "mkv" | "zip") => {
                            media_filename = file_name.clone();
                            Path::new(".").join("SavedFile").join(file_name)
                        },
                        _ => return HttpResponse::BadRequest().body("Invalid file type or extension."),
                    };
                    if create_file(&file_path, &mut field).await.is_err() {
                        return HttpResponse::InternalServerError().finish();
                    }
                } else {
                    return HttpResponse::BadRequest().body("Invalid payload.")
                }
            }
        } else {
            return HttpResponse::BadRequest().body("Invalid payload.");
        }
    }
    if let Some(model_type) = model_type {
        let new_task = Task::new(uuid, model_filename, media_filename, model_type).await;
        FileManager::add_pre_process_task(new_task).await;
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::BadRequest().finish()
    }
}

fn get_field_name(content_disposition: &ContentDisposition) -> Option<String> {
    match content_disposition.get_name() {
        Some(field_name) => Some(field_name.to_string()),
        _ => None,
    }
}

fn get_file_name(content_disposition: &ContentDisposition) -> Option<String> {
    match content_disposition.get_filename() {
        Some(file_name) => Some(file_name.to_string()),
        _ => None
    }
}

async fn parse_model_type(field: &mut Field) -> Option<ModelType> {
    let data = field.next().await?.ok()?;
    let model_type = String::from_utf8_lossy(&data).to_string();
    ModelType::from_str(&*model_type).ok()
}

async fn create_file(file_path: &PathBuf, field: &mut Field) -> Result<(), ()>{
    let mut file = File::create(&file_path).await.map_err(|_| ())?;
    while let Some(chunk) = field.next().await {
        let data = chunk.map_err(|_| ())?;
        file.write_all(&data).await.map_err(|_| ())?;
    }
    Ok(())
}
