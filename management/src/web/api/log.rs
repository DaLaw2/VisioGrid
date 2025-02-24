use crate::utils::logging::Logger;
use actix_web::{get, web, HttpResponse, Responder, Scope};
use chrono::{DateTime, Local};
use uuid::Uuid;

pub fn initialize() -> Scope {
    web::scope("/log")
        .service(system_log)
        .service(system_log_since)
        .service(agent_log)
        .service(agent_log_since)
}

#[get("/system_log")]
async fn system_log() -> impl Responder {
    let system_log = Logger::get_system_logs().await
        .into_iter().map(|log| log.to_colored_string()).collect::<Vec<String>>();
    HttpResponse::Ok().json(web::Json(system_log))
}

#[get("/system_log/since/{since}")]
async fn system_log_since(since: web::Path<String>) -> impl Responder {
    match parse_datetime(&since.into_inner()) {
        Ok(since_time) => {
            let logs = Logger::get_system_logs_since(since_time).await
                .into_iter().map(|log| log.to_colored_string()).collect::<Vec<String>>();
            HttpResponse::Ok().json(web::Json(logs))
        }
        Err(_) => HttpResponse::BadRequest().body("Invalid datetime format."),
    }
}

#[get("/{agent_id}")]
async fn agent_log(agent_id: web::Path<Uuid>) -> impl Responder {
    match Logger::get_agent_logs(agent_id.into_inner()).await {
        Some(agent_log) => {
            let agent_log = agent_log
                .into_iter().map(|log| log.to_colored_string()).collect::<Vec<String>>();
            HttpResponse::Ok().json(web::Json(agent_log))
        }
        None => HttpResponse::BadRequest().body("agent not found.")
    }
}

#[get("/{agent_id}/since/{since}")]
async fn agent_log_since(argument: web::Path<(Uuid, String)>) -> impl Responder {
    let (agent_id, since_str) = argument.into_inner();
    match parse_datetime(&since_str) {
        Ok(since_time) => {
            match Logger::get_agent_logs_since(agent_id, since_time).await {
                Some(logs) => {
                    let logs = logs
                        .into_iter().map(|log| log.to_colored_string()).collect::<Vec<String>>();
                    HttpResponse::Ok().json(web::Json(logs))
                },
                None => HttpResponse::BadRequest().body("agent not found."),
            }
        }
        Err(_) => HttpResponse::BadRequest().body("Invalid datetime format"),
    }
}

fn parse_datetime(datetime_str: &str) -> Result<DateTime<Local>, String> {
    DateTime::parse_from_rfc3339(datetime_str)
        .map_err(|_| "Invalid datetime format".to_string())
        .map(|dt| dt.with_timezone(&Local))
}
