use uuid::Uuid;
use tokio::time::sleep;
use std::time::Duration;
use lazy_static::lazy_static;
use actix_web::{App, HttpServer};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::manager::node::Node;
use crate::utils::config::Config;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::node_cluster::NodeCluster;
use crate::manager::file_manager::FileManager;
use crate::connection::socket::node_socket::NodeSocket;
use crate::web::page::{config, inference, javascript, log};

lazy_static!{
    static ref GLOBAL_SERVER: RwLock<Server> = RwLock::new(Server::new());
}

pub struct Server {
    terminate: bool,
}

impl Server {
    fn new() -> Self {
        Self {
            terminate: false,
        }
    }

    pub async fn instance() -> RwLockReadGuard<'static, Self> {
        GLOBAL_SERVER.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        GLOBAL_SERVER.write().await
    }

    pub async fn run() {
        Config::now().await;
        FileManager::run().await;
        NodeCluster::run().await;
        Self::register_node().await;
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
                    Logger::append_system_log(LogLevel::ERROR, format!("Server: Http service bind port failed.\nReason: {}", err)).await;
                    sleep(Duration::from_millis(config.internal_timestamp)).await;
                    continue;
                },
            }
        };
        Logger::append_system_log(LogLevel::INFO, "Server: Web service ready.".to_string()).await;
        Logger::append_system_log(LogLevel::INFO, "Server: Online.".to_string()).await;
        if let Err(err) = http_server.run().await {
            Logger::append_system_log(LogLevel::ERROR, format!("Server: Error while Http service running.\nReason: {}", err)).await
        }
    }

    pub async fn terminate() {
        Logger::append_system_log(LogLevel::INFO, "Server: Terminating.".to_string()).await;
        NodeCluster::terminate().await;
        FileManager::terminate().await;
        Self::instance_mut().await.terminate = true;
        Logger::append_system_log(LogLevel::INFO, "Server: Termination complete.".to_string()).await;
    }

    async fn register_node() {
        tokio::spawn(async {
            while !Self::instance().await.terminate {
                let node_id = Uuid::new_v4();
                let mut node_socket = NodeSocket::new().await;
                let (socket_stream, node_ip) = node_socket.get_connection().await;
                let node = Node::new(node_id, socket_stream).await;
                if let Some(node) = node {
                    NodeCluster::add_node(node).await;
                    Logger::append_system_log(LogLevel::INFO, format!("Server: Node connected.\nIp: {}, Allocate node ID: {}", node_ip, node_id)).await;
                }
            }
        });
    }
}
