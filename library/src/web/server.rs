use uuid::Uuid;
use lazy_static::lazy_static;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::manager::node::Node;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::node_cluster::NodeCluster;
use crate::connection::socket::node_socket::NodeSocket;

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
        tokio::spawn(async move {
            loop {
                if Self::instance().await.terminate {
                    return;
                }
                let node_id = Uuid::new_v4();
                let mut node_socket = NodeSocket::new().await;
                let (socket_stream, node_ip) = node_socket.get_connection().await;
                let node = Node::new(node_id, socket_stream).await;
                if let Some(node) = node {
                    NodeCluster::add_node(node).await;
                    Logger::append_system_log(LogLevel::INFO, format!("Node connected.\nIp: {}, Allocate node ID: {}", node_ip, node_id)).await;
                }
            }
        });
        Logger::append_system_log(LogLevel::INFO, "Server: Online.".to_string()).await;
    }

    pub async fn terminate() {
        Logger::append_system_log(LogLevel::INFO, "Server: Terminating.".to_string()).await;
        Self::instance_mut().await.terminate = true;
        Logger::append_system_log(LogLevel::INFO, "Server: Termination complete.".to_string()).await;
    }
}
