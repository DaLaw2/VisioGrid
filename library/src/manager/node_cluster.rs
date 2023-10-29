use lazy_static::lazy_static;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{Mutex, MutexGuard};
use tokio::time::sleep;
use crate::manager::node::Node;

lazy_static! {
    static ref GLOBAL_CLUSTER: Mutex<NodeCluster> = Mutex::new(NodeCluster::new());
}

pub struct NodeCluster {
    size: usize,
    nodes: HashMap<usize, Node>,
    performance: Vec<(usize, f64)>,
}

impl NodeCluster {
    fn new() -> Self {
        Self {
            size: 0_usize,
            nodes: HashMap::new(),
            performance: Vec::new(),
        }
    }

    pub async fn run() {
        tokio::spawn(async {
            loop {
                {
                    let mut node_cluster = GLOBAL_CLUSTER.lock().await;
                    let mut performance: Vec<(usize, f64)> = node_cluster.nodes.iter().map(|(&key, node)| (key, node.idle_performance.vram)).collect();
                    performance.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                    node_cluster.performance = performance;
                }
                sleep(Duration::from_millis(100)).await;
            }
        });
    }

    pub async fn instance() -> MutexGuard<'static, NodeCluster> {
        GLOBAL_CLUSTER.lock().await
    }

    pub fn add_node(&mut self, node: Node) {
        let node_id = node.get_id();
        if self.nodes.contains_key(&node_id) {
            return;
        }
        self.nodes.insert(node_id, node);
        self.size += 1;
    }

    pub fn remove_node(&mut self, node_id: usize) -> Option<Node> {
        let node = self.nodes.remove(&node_id);
        match node {
            Some(_) => self.size -= 1,
            None => {}
        }
        node
    }

    pub fn get_node(&mut self, node_id: usize) -> Option<&mut Node> {
        self.nodes.get_mut(&node_id)
    }

    pub fn size(&self) -> usize {
        self.size
    }
}
