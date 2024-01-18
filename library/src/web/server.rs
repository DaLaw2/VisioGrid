use uuid::Uuid;
use crate::manager::node::Node;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::node_cluster::NodeCluster;
use crate::connection::socket::node_socket::NodeSocket;

pub struct Server {
    node_socket: NodeSocket,
}

impl Server {
    async fn new() -> Self {
        Logger::append_system_log(LogLevel::INFO, "Server online.".to_string()).await;
        Self {
            node_socket: NodeSocket::new().await,
        }
    }

    pub fn run(mut self) {
        tokio::spawn(async move {
            loop {
                let node_id = Uuid::new_v4();
                let (socket_stream, node_ip) = self.node_socket.get_connection().await;
                let node = Node::new(node_id, socket_stream).await;
                if let Some(node) = node {
                    NodeCluster::add_node(node).await;
                    Logger::append_system_log(LogLevel::INFO, format!("Node connected.\nIp: {}, Allocate node ID: {}", node_ip, node_id)).await;
                }
            }
        });
    }
}
