use lazy_static::lazy_static;
use std::collections::HashMap;
use tokio::sync::{Mutex, MutexGuard};
use crate::manager::node::Node;

lazy_static! {
    static ref GLOBAL_CLUSTER: Mutex<NodeCluster> = Mutex::new(NodeCluster::new());
}

pub struct NodeCluster {
    nodes: HashMap<usize, Node>
}

impl NodeCluster {
    fn new() -> NodeCluster {
        NodeCluster {
            nodes: HashMap::new()
        }
    }

    pub async fn instance() -> MutexGuard<'static, NodeCluster> {
        GLOBAL_CLUSTER.lock().await
    }

    pub fn add_node(&mut self, node: Node) {
        let node_id = node.get_node_id();
        if self.nodes.contains_key(&node_id) {
            return;
        }
        self.nodes.insert(node_id, node);
    }

    pub fn remove_node(&mut self, node_id: usize) -> Option<Node> {
        self.nodes.remove(&node_id)
    }

    pub fn get_node(&mut self, node_id: usize) -> Option<&mut Node> {
        self.nodes.get_mut(&node_id)
    }
}
