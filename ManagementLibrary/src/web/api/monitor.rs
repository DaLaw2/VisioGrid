use uuid::Uuid;
use actix_web_actors::ws::start;
use actix_web::{get, HttpResponse, Error};
use actix_web::{HttpRequest, Responder, Scope, web};
use crate::management::monitor::Monitor;
use crate::management::agent_manager::AgentManager;
use crate::web::utils::performance_websocket::PerformanceWebSocket;

pub fn initialize() -> Scope {
    web::scope("/monitor")
        .service(get_agent_list)
        .service(get_information)
        .service(get_performance)
        .service(websocket)
}

#[get("/get/agent_list")]
async fn get_agent_list() -> impl Responder {
    let agents = AgentManager::get_agents_uuid().await;
    match serde_json::to_string(&agents) {
        Ok(json) => HttpResponse::Ok().json(web::Json(json)),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/get/information/{target}")]
async fn get_information(target: web::Path<String>) -> impl Responder {
    let target = target.into_inner();
    if target == "system" {
        let system_information = Monitor::get_system_info().await;
        match serde_json::to_string(&system_information) {
            Ok(json) => HttpResponse::Ok().json(web::Json(json)),
            Err(_) => HttpResponse::InternalServerError().finish(),
        }
    } else {
        match Uuid::parse_str(&target) {
            Ok(agent_id) => {
                match AgentManager::get_agent_information(agent_id).await {
                    Some(agent_information) => {
                        match serde_json::to_string(&agent_information) {
                            Ok(json) => HttpResponse::Ok().json(web::Json(json)),
                            Err(_) => HttpResponse::InternalServerError().finish(),
                        }
                    },
                    None => HttpResponse::NotFound().finish(),
                }
            },
            Err(_) => HttpResponse::BadRequest().finish(),
        }
    }
}

#[get("/get/performance/{target}")]
async fn get_performance(target: web::Path<String>) -> impl Responder {
    let target = target.into_inner();
    if target == "system" {
        let system_performance = Monitor::get_performance().await;
        match serde_json::to_string(&system_performance) {
            Ok(json) => HttpResponse::Ok().json(web::Json(json)),
            Err(_) => HttpResponse::InternalServerError().finish(),
        }
    } else {
        match Uuid::parse_str(&target) {
            Ok(agent_id) => {
                match AgentManager::get_agent_performance(agent_id).await {
                    Some(agent_performance) => {
                        match serde_json::to_string(&agent_performance) {
                            Ok(json) => HttpResponse::Ok().json(web::Json(json)),
                            Err(_) => HttpResponse::InternalServerError().finish(),
                        }
                    },
                    None => HttpResponse::NotFound().finish(),
                }
            },
            Err(_) => HttpResponse::BadRequest().finish(),
        }
    }
}

#[get("/websocket/performance/{target}")]
async fn websocket(req: HttpRequest, stream: web::Payload, target: web::Path<String>) -> Result<HttpResponse, Error> {
    let target = target.into_inner();
    let target_type;
    let agent_id: Option<Uuid>;
    if target == "system" {
        target_type = "system";
        agent_id = None;
    } else {
        match Uuid::parse_str(&target) {
            Ok(id) => {
                if !AgentManager::is_agent_exists(id).await {
                    return Ok(HttpResponse::NotFound().finish());
                }
                target_type = "agent";
                agent_id = Some(id);
            },
            Err(_) => return Ok(HttpResponse::BadRequest().finish()),
        }
    }
    let websocket = PerformanceWebSocket {
        interval: None,
        target_type: target_type.to_string(),
        agent_id,
    };
    start(websocket, &req, stream)
}
