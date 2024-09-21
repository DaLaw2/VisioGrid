use std::path::PathBuf;
use actix_files::NamedFile;
use actix_web::{get, web, HttpResponse, Responder, Scope, HttpRequest};
use actix_web::http::header::{ContentDisposition, DispositionParam, DispositionType};
use uuid::Uuid;
use crate::management::task_manager::TaskManager;

pub fn initialize() -> Scope {
    web::scope("/task")
        .service(processing_tasks)
        .service(success_tasks)
        .service(failed_tasks)
        .service(download_result)
}

#[get("/processing_tasks")]
async fn processing_tasks() -> impl Responder {
    let tasks = TaskManager::get_processing_tasks().await;
    HttpResponse::Ok().json(web::Json(tasks))
}

#[get("/success_tasks")]
async fn success_tasks() -> impl Responder {
    let tasks = TaskManager::get_success_tasks().await;
    HttpResponse::Ok().json(web::Json(tasks))
}

#[get("/failed_tasks")]
async fn failed_tasks() -> impl Responder {
    let tasks = TaskManager::get_fail_tasks().await;
    HttpResponse::Ok().json(web::Json(tasks))
}

#[get("/download/{uuid}")]
async fn download_result(req: HttpRequest, uuid: web::Path<Uuid>) -> impl Responder {
    let result = TaskManager::clone_success_task(&uuid.into_inner()).await;
    match result {
        Some(task) => {
            #[cfg(target_os = "linux")]
            let file_path = PathBuf::from(format!("./Result/{}", task.media_file_name));
            #[cfg(target_os = "windows")]
            let file_path = PathBuf::from(format!(".\\Result\\{}", task.media_file_name));
            match NamedFile::open_async(&file_path).await {
                Ok(named_file) => {
                    let cd = ContentDisposition {
                        disposition: DispositionType::Attachment,
                        parameters: vec![
                            DispositionParam::Filename(task.media_file_name.clone()),
                        ],
                    };
                    named_file
                        .set_content_disposition(cd)
                        .set_content_type(mime_guess::from_path(&file_path).first_or_octet_stream())
                        .into_response(&req)
                }
                Err(_) => HttpResponse::NotFound().finish()
            }
        }
        None => HttpResponse::NotFound().finish(),
    }
}
