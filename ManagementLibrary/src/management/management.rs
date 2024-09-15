use uuid::Uuid;
use tokio::time::sleep;
use std::time::Duration;
use lazy_static::lazy_static;
use actix_web::{App, HttpServer};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::utils::logging::*;
use crate::utils::config::Config;
use crate::management::agent::Agent;
use crate::management::monitor::Monitor;
use crate::management::file_manager::FileManager;
use crate::management::agent_manager::AgentManager;
use crate::connection::socket::agent_socket::AgentSocket;
use crate::web::api::{config, inference, javascript, log, misc, monitor};

lazy_static! {
    static ref MANAGEMENT: RwLock<Management> = RwLock::new(Management::new());
}

pub struct Management {
    terminate: bool,
}

impl Management {
    fn new() -> Self {
        Self {
            terminate: false,
        }
    }

    pub async fn instance() -> RwLockReadGuard<'static, Self> {
        MANAGEMENT.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        MANAGEMENT.write().await
    }

    pub async fn run() {
        Config::now().await;
        FileManager::run().await;
        Monitor::run().await;
        Self::register_agent().await;
        let http_server = loop {
            let config = Config::now().await;
            let http_server = HttpServer::new(|| {
                App::new()
                    .service(config::initialize())
                    .service(inference::initialize())
                    .service(javascript::initialize())
                    .service(log::initialize())
                    .service(misc::initialize())
                    .service(monitor::initialize())
            }).bind(format!("0.0.0.0:{}", config.http_server_bind_port));
            match http_server {
                Ok(http_server) => break http_server,
                Err(err) => {
                    logging_critical!("Management", "Failed to bind port", format!("Err: {err}"));
                    sleep(Duration::from_secs(config.bind_retry_duration)).await;
                    continue;
                },
            }
        };
        logging_information!("Management", "Web service ready");
        logging_information!("Management", "Online now");
        if let Err(err) = http_server.run().await {
            logging_emergency!("Management", "An error occurred while running the web service", format!("Err: {err}"));
        }
    }

    pub async fn terminate() {
        logging_information!("Management", "Termination in progress");
        Self::instance_mut().await.terminate = true;
        Monitor::terminate().await;
        FileManager::terminate().await;
        logging_information!("Management", "Termination complete");
    }

    async fn register_agent() {
        tokio::spawn(async {
            let mut agent_socket = AgentSocket::new().await;
            while !Self::instance().await.terminate {
                let agent_id = Uuid::new_v4();
                let (socket_stream, agent_ip) = agent_socket.get_connection().await;
                let agent = Agent::new(agent_id, socket_stream).await;
                match agent {
                    Ok(agent) => {
                        AgentManager::add_agent(agent).await;
                        logging_information!("Management", format!("Agent {agent_ip} is connected"));
                    },
                    Err(entry) => logging_entry!(agent_id, entry),
                }
            }
        });
    }
}
