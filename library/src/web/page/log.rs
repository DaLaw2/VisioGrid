use uuid::Uuid;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use actix_web::{get, web, Scope, Responder, HttpResponse};
use crate::utils::logger::Logger;
use crate::utils::static_files::StaticFiles;

pub fn initialize() -> Scope {
    web::scope("/log")
        .service(page)
        .service(system_log)
        .service(update_system_log)
        .service(node_log)
        .service(node_log_update)
}

#[get("")]
async fn page() -> impl Responder {
    let html = StaticFiles::get("log.html").expect("File not found in static files.").data;
    HttpResponse::Ok().content_type("text/html").body(html)
}

#[get("/system_log")]
async fn system_log() -> impl Responder {
    let system_log = Logger::get_system_logs().await;
    let system_log_string = Logger::format_logs(&system_log);
    HttpResponse::Ok().body(system_log_string)
}

#[get("/system_log/update/{since}")]
async fn update_system_log(path: web::Path<String>) -> impl Responder {
    match parse_datetime(&path.into_inner()) {
        Ok(since_time) => {
            let logs = Logger::get_system_logs_since(since_time).await;
            let log_string = Logger::format_logs(&logs);
            HttpResponse::Ok().body(log_string)
        },
        Err(_) => HttpResponse::BadRequest().body("Invalid datetime format.")
    }
}

#[get("/{node_id}")]
async fn node_log(node_id: web::Path<Uuid>) -> impl Responder {
    match Logger::get_node_logs(node_id.into_inner()).await {
        Some(node_log) => {
            let log_string = Logger::format_logs(&node_log);
            HttpResponse::Ok().body(log_string)
        }
        None => HttpResponse::BadRequest().body("Node not found.")
    }
}

#[get("/{node_id}/update/{since}")]
async fn node_log_update(path: web::Path<(Uuid, String)>) -> impl Responder {
    let (node_id, since_str) = path.into_inner();
    match parse_datetime(&since_str) {
        Ok(since_time) => {
            match Logger::get_node_logs_since(node_id, since_time).await {
                Some(logs) => {
                    let log_string = Logger::format_logs(&logs);
                    HttpResponse::Ok().body(log_string)
                }
                None => HttpResponse::BadRequest().body("Node not found.")
            }
        },
        Err(_) => HttpResponse::BadRequest().body("Invalid datetime format")
    }
}

fn parse_datetime(datetime_str: &str) -> Result<DateTime<Local>, String> {
    NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d-%H-%M-%S")
        .map_err(|_| "Invalid datetime format".to_string())
        .and_then(|naive_date_time| {
            Local.from_local_datetime(&naive_date_time)
                .single()
                .ok_or("Invalid local datetime".to_string())
        })
}
