use crate::management::task_manager::TaskManager;
use crate::management::utils::inference_argument::InferenceArgument;
use crate::management::utils::task::Task;
use actix_multipart::{Field, Multipart};
use actix_web::http::header::ContentDisposition;
use actix_web::{post, web, HttpResponse, Responder, Scope};
use futures::{StreamExt, TryStreamExt};
use sanitize_filename::sanitize;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

pub fn initialize() -> Scope {
    web::scope("/inference")
        .service(save_files)
}

#[post("/request")]
async fn save_files(mut payload: Multipart) -> impl Responder {
    let uuid = Uuid::new_v4();
    let mut inference_argument = None;
    let mut model_file_name = String::new();
    let mut media_file_name = String::new();
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = match field.content_disposition() {
            Some(content_disposition) => content_disposition,
            None => return HttpResponse::InternalServerError().finish(),
        };
        if let Some(field_name) = get_field_name(&content_disposition) {
            if field_name == "inferenceArgument" {
                inference_argument = parse_inference_argument(&mut field).await;
            } else {
                if let Some(mut file_name) = get_file_name(&content_disposition) {
                    let sanitized_file_name = sanitize(file_name);
                    if sanitized_file_name.is_empty() {
                        return HttpResponse::BadRequest().body("Invalid filename.");
                    }
                    file_name = format!("{}_{}", uuid, sanitized_file_name);
                    let file_extension = Path::new(&file_name).extension()
                        .and_then(|os_str| os_str.to_str()).unwrap_or("");
                    let save_path = match (&*field_name, file_extension) {
                        ("modelFile", "pt" | "pth" | "onnx") => {
                            model_file_name = file_name.clone();
                            #[cfg(target_os = "linux")]
                            { PathBuf::from(format!("./SavedModel/{}", file_name)) }
                            #[cfg(target_os = "windows")]
                            { PathBuf::from(format!(".\\SavedModel\\{}", file_name)) }
                        },
                        ("mediaFile", "png" | "jpg" | "jpeg" | "mp4" | "avi" | "mkv" | "zip") => {
                            media_file_name = file_name.clone();
                            #[cfg(target_os = "linux")]
                            { PathBuf::from(format!("./SavedFile/{}", file_name)) }
                            #[cfg(target_os = "windows")]
                            { PathBuf::from(format!(".\\SavedFile\\{}", file_name)) }
                        },
                        _ => return HttpResponse::BadRequest().body("Invalid file type or extension."),
                    };
                    if create_file(&save_path, &mut field).await.is_err() {
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
    if inference_argument.is_none() || model_file_name.is_empty() || media_file_name.is_empty() {
        return HttpResponse::BadRequest().body("Invalid payload.")
    }
    // Have checked above
    let inference_argument = inference_argument.unwrap();
    let new_task = Task::new(uuid, model_file_name, media_file_name, inference_argument).await;
    TaskManager::add_task(new_task).await;
    HttpResponse::Ok().finish()
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

async fn parse_inference_argument(field: &mut Field) -> Option<InferenceArgument> {
    let mut data = Vec::new();
    while let Some(chunk) = field.next().await {
        let bytes = chunk.ok()?;
        data.extend_from_slice(&bytes);
    }
    let json_str = String::from_utf8_lossy(&data).to_string();
    serde_json::from_str(&json_str).ok()
}

async fn create_file(save_path: &PathBuf, field: &mut Field) -> Result<(), ()>{
    let mut file = File::create(&save_path).await.map_err(|_| ())?;
    while let Some(chunk) = field.next().await {
        let data = chunk.map_err(|_| ())?;
        file.write_all(&data).await.map_err(|_| ())?;
    }
    Ok(())
}
