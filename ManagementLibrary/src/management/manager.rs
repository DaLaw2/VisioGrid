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
use crate::web::page::{config, inference, javascript, log};

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
        Config::now().await;
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
            }).bind(format!("127.0.0.1:{}", config.http_server_bind_port));
            match http_server {
                Ok(http_server) => break http_server,
                Err(err) => {
                    Logger::append_system_log(LogLevel::ERROR, format!("Management: Http service bind port failed.\nReason: {}", err)).await;
                    sleep(Duration::from_millis(config.internal_timestamp)).await;
                    continue;
                },
            }
        };
        Logger::append_system_log(LogLevel::INFO, "Management: Web service ready.".to_string()).await;
        Logger::append_system_log(LogLevel::INFO, "Management: Online.".to_string()).await;
        if let Err(err) = http_server.run().await {
            Logger::append_system_log(LogLevel::ERROR, format!("Management: Error while Http service running.\nReason: {}", err)).await
        }
    }

    pub async fn terminate() {
        Logger::append_system_log(LogLevel::INFO, "Management: Terminating.".to_string()).await;
        AgentManager::terminate().await;
        FileManager::terminate().await;
        Self::instance_mut().await.terminate = true;
        Logger::append_system_log(LogLevel::INFO, "Management: Termination complete.".to_string()).await;
    }

    async fn register_agent() {
        tokio::spawn(async {
            while !Self::instance().await.terminate {
                let agent_id = Uuid::new_v4();
                let mut agent_socket = AgentSocket::new().await;
                let (socket_stream, agent_ip) = agent_socket.get_connection().await;
                let agent = Agent::new(agent_id, socket_stream).await;
                if let Some(agent) = agent {
                    AgentManager::add_agent(agent).await;
                    Logger::append_system_log(LogLevel::INFO, format!("Management: Agent connected.\nIp: {}, Allocate agent ID: {}", agent_ip, agent_id)).await;
                }
            }
        });
    }
}
