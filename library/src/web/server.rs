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
        Self {
            id_manager: IDManager::new(),
            node_socket: NodeSocket::new().await,
        }
    }

    pub fn run(mut self) {
        tokio::spawn(async move {
            loop {
                let node_id = self.id_manager.allocate_id();
                let (socket_stream, node_ip) = self.node_socket.get_connection().await;
                let node = Node::new(node_id, socket_stream);
                NodeCluster::instance().await.add_node(node);
                Logger::instance().await.append_global_log(LogLevel::INFO, format!("Node {} connected.", node_ip));
            }
        });
    }
}
