use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use actix_multipart::Multipart;
use std::path::{Path, PathBuf};
use sanitize_filename::sanitize;
use futures::{self, StreamExt, TryStreamExt};
use actix_web::{get, post, web, Scope, Result, Error, HttpRequest, HttpResponse, Responder};
use crate::utils::static_files::StaticFiles;
use crate::web::utils::response::OperationStatus;
use crate::manager::task::file_manager::FileManager;
use crate::manager::task::definition::{InferenceType, Task};

pub fn initialize() -> Scope {
    web::scope("/inference")
        .service(inference)
        .service(save_files)
}

#[get("")]
async fn inference() -> impl Responder {
    let html = StaticFiles::get("inference.html").expect("File not found in static files.").data;
    let response = HttpResponse::Ok().content_type("text/html").body(html);
    response
}

#[post("/save_file")]
async fn save_files(req: HttpRequest, mut payload: Multipart) -> Result<HttpResponse, Error> {
    let mut model_type = String::new();
    let mut model_filename = String::new();
    let mut inference_filename = String::new();
    let client_ip = match req.connection_info().peer_addr() {
        Some(ip) => ip.to_string(),
        None => return Ok(HttpResponse::InternalServerError().json(web::Json(OperationStatus::new(false, Some("Unknown ip.".to_string())))))
    };
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
        file_name = format!("{}_{}_{}", client_ip, model_type, file_name);
        let file_extension = Path::new(&file_name).extension().and_then(|os_str| os_str.to_str()).unwrap_or("");
        let file_path = match (field_name, file_extension) {
            ("cfgFile" | "weightsFile" | "namesFile" | "ptFile" | "h5File" | "onnxFile", "cfg" | "weights" | "names" | "pt" | "pth" | "h5" | "onnx") => {
                model_filename = file_name.clone();
                "./SavedModel"
            },
            ("yoloInferenceFile" | "pytorchInferenceFile" | "tensorflowInferenceFile" | "onnxInferenceFile" | "defaultInferenceFile", "png" | "jpg" | "jpeg" | "gif" | "mp4" | "wav" | "avi" | "mkv" | "zip") => {
                inference_filename = file_name.clone();
                "./SavedFile"
            },
            _ => return Ok(HttpResponse::BadRequest().json(web::Json(OperationStatus::new(false, Some("Invalid file type or extension.".to_string())))))
        };
        let file_path: PathBuf = format!("{}/{}", file_path, file_name).into();
        let mut file = File::create(&file_path).await?;
        while let Some(chunk) = field.next().await {
            match chunk {
                Ok(data) => file.write_all(&data).await?,
                Err(err) => return Ok(HttpResponse::InternalServerError().json(web::Json(OperationStatus::new(false, Some(format!("{}", err))))))
            }
        }
    }
    let new_task = Task::new(client_ip, model_filename, inference_filename, match &*model_type {
        "YOLO" => InferenceType::YOLO,
        "PyTorch" => InferenceType::PyTorch,
        "TensorFlow" => InferenceType::TensorFlow,
        "ONNX" => InferenceType::ONNX,
        _ => InferenceType::Default
    }).await;
    FileManager::add_task(new_task).await;
    Ok(HttpResponse::Ok().json(OperationStatus::new(true, None)))
}
