use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use lazy_static::lazy_static;
use std::collections::HashMap;
use crate::manager::node::Node;

lazy_static! {
    static ref GLOBAL_CLUSTER: Mutex<NodeCluster> = Mutex::new(NodeCluster::new());
}

pub struct NodeCluster {
    size: usize,
    nodes: HashMap<usize, Node>,
    vram_sorting: Vec<(usize, f64)>,
}

impl NodeCluster {
    fn new() -> Self {
        Self {
            size: 0_usize,
            nodes: HashMap::new(),
            vram_sorting: Vec::new(),
        }
    }

    pub async fn run() {
        tokio::spawn(async {
            loop {
                {
                    let mut node_cluster = GLOBAL_CLUSTER.lock().await;
                    let mut vram: Vec<(usize, f64)> = node_cluster.nodes.iter().map(|(&key, node)| (key, node.idle_performance.vram)).collect();
                    vram.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                    node_cluster.vram_sorting = vram;
                }
                sleep(Duration::from_millis(100)).await;
            }
        });
    }

    pub async fn add_node(node: Node) {
        let mut node_cluster = GLOBAL_CLUSTER.lock().await;
        let node_id = node.get_id();
        if node_cluster.nodes.contains_key(&node_id) {
            return;
        }
        node_cluster.nodes.insert(node_id, node);
        node_cluster.size += 1;
    }

    pub async fn remove_node(node_id: usize) -> Option<Node> {
        let mut node_cluster = GLOBAL_CLUSTER.lock().await;
        let node = node_cluster.nodes.remove(&node_id);
        match node {
            Some(_) => node_cluster.size -= 1,
            None => {}
        }
        node
    }

    pub async fn get_node(&mut self, node_id: usize) -> Option<&mut Node> {
        self.nodes.get_mut(&node_id)
    }

    pub async fn get_vram_sorting() -> Vec<(usize, f64)> {
        let node_cluster = GLOBAL_CLUSTER.lock().await;
        node_cluster.vram_sorting.clone()
    }

    pub async fn size() -> usize {
        let node_cluster = GLOBAL_CLUSTER.lock().await;
        node_cluster.size
    }
}
