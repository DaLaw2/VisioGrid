use crate::manager::node::Node;
use crate::utils::id_manager::IDManager;
use crate::utils::logger::{Logger, LogLevel};
use crate::manager::node_cluster::NodeCluster;
use crate::connection::socket::node_socket::NodeSocket;

pub struct Server {
    id_manager: IDManager,
    node_socket: NodeSocket,
}

impl Server {
    async fn new() -> Self {
        let server = Self {
            id_manager: IDManager::new(),
            node_socket: NodeSocket::new().await,
        };
        Logger::append_system_log(LogLevel::INFO, "Server online.".to_string()).await;
        server
    }

    pub fn run(mut self) {
        tokio::spawn(async move {
            loop {
                let node_id = self.id_manager.allocate_id();
                let (socket_stream, node_ip) = self.node_socket.get_connection().await;
                let node = Node::new(node_id, socket_stream).await;
                match node {
                    Some(node) => {
                        NodeCluster::add_node(node).await;
                        Logger::append_system_log(LogLevel::INFO, format!("Node connected.\nIp: {}, Allocate node ID: {}", node_ip, node_id)).await;
                    },
                    None => self.id_manager.free_id(node_id),
                }
            }
        });
    }
}
