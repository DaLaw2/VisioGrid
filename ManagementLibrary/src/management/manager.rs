use uuid::Uuid;
use tokio::time::sleep;
use std::time::Duration;
use lazy_static::lazy_static;
use actix_web::{App, HttpServer};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::utils::config::Config;
use crate::management::agent::Agent;
use crate::utils::logger::{Logger, LogLevel};
use crate::management::file_manager::FileManager;
use crate::management::agent_manager::AgentManager;
use crate::connection::socket::agent_socket::AgentSocket;
use crate::{logging_entry, logging_error, logging_info};
use crate::web::page::{config, inference, javascript, log, misc};

lazy_static!{
    static ref MANAGEMENT: RwLock<Manager> = RwLock::new(Manager::new());
}

pub struct Manager {
    terminate: bool,
}

impl Manager {
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
        FileManager::run().await;
        AgentManager::run().await;
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
            }).bind(format!("127.0.0.1:{}", config.http_server_bind_port));
            match http_server {
                Ok(http_server) => break http_server,
                Err(err) => {
                    logging_error!(format!("Management: Http service bind port failed.\nReason: {err}"));
                    sleep(Duration::from_secs(config.bind_retry_duration)).await;
                    continue;
                },
            }
        };
        logging_info!("Management: Web service ready.");
        logging_info!("Management: Online.");
        if let Err(err) = http_server.run().await {
            logging_error!(format!("Management: Error while web service running.\nReason: {err}"));
        }
    }

    pub async fn terminate() {
        logging_info!("Management: Terminating.");
        AgentManager::terminate().await;
        FileManager::terminate().await;
        Self::instance_mut().await.terminate = true;
        logging_info!("Management: Termination complete.");
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
                        logging_info!(format!("Management: Agent connected.\nIp: {agent_ip}, Allocate agent ID: {agent_id}"));
                    },
                    Err(entry) => logging_entry!(agent_id, entry),
                }
            }
        });
    }
}
