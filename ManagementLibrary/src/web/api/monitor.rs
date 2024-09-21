use crate::management::agent_manager::AgentManager;
use crate::management::monitor::Monitor;
use crate::web::utils::performance_websocket::PerformanceWebSocket;
use actix_web::{get, Error, HttpResponse};
use actix_web::{web, HttpRequest, Responder, Scope};
use actix_web_actors::ws::start;
use uuid::Uuid;

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
    HttpResponse::Ok().json(web::Json(agents))
}

#[get("/get/information/{target}")]
async fn get_information(target: web::Path<String>) -> impl Responder {
    let target = target.into_inner();
    if target == "system" {
        let system_information = Monitor::get_system_info().await;
        HttpResponse::Ok().json(web::Json(system_information))
    } else {
        match Uuid::parse_str(&target) {
            Ok(agent_id) => {
                match AgentManager::get_agent_information(agent_id).await {
                    Some(agent_information) => HttpResponse::Ok().json(web::Json(agent_information)),
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
        HttpResponse::Ok().json(web::Json(system_performance))
    } else {
        match Uuid::parse_str(&target) {
            Ok(agent_id) => {
                match AgentManager::get_agent_performance(agent_id).await {
                    Some(agent_performance) => HttpResponse::Ok().json(web::Json(agent_performance)),
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
