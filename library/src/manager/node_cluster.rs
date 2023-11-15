use tokio::time::sleep;
use std::time::Duration;
use lazy_static::lazy_static;
use std::collections::HashMap;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::manager::node::Node;

lazy_static! {
    static ref GLOBAL_CLUSTER: RwLock<NodeCluster> = RwLock::new(NodeCluster::new());
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

    pub async fn instance() -> RwLockReadGuard<'static, NodeCluster> {
        GLOBAL_CLUSTER.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, NodeCluster> {
        GLOBAL_CLUSTER.write().await
    }

    pub async fn run() {
        tokio::spawn(async {
            loop {
                {
                    let mut node_cluster = GLOBAL_CLUSTER.write().await;
                    let mut vram: Vec<(usize, f64)> = node_cluster.nodes.iter().map(|(&key, node)| (key, node.idle_unused.vram)).collect();
                    vram.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                    node_cluster.vram_sorting = vram;
                }
                sleep(Duration::from_millis(100)).await;
            }
        });
    }

    pub async fn add_node(node: Node) {
        let mut node_cluster = GLOBAL_CLUSTER.write().await;
        let node_id = node.id;
        if node_cluster.nodes.contains_key(&node_id) {
            return;
        }
        node_cluster.nodes.insert(node_id, node);
        node_cluster.size += 1;
    }

    pub async fn remove_node(node_id: usize) -> Option<Node> {
        let mut node_cluster = GLOBAL_CLUSTER.write().await;
        let node = node_cluster.nodes.remove(&node_id);
        match node {
            Some(_) => node_cluster.size -= 1,
            None => {}
        }
        node
    }

    pub fn get_node(&self, node_id: usize) -> Option<&Node> {
        self.nodes.get(&node_id)
    }

    pub fn get_node_mut(&mut self, node_id: usize) -> Option<&mut Node> {
        self.nodes.get_mut(&node_id)
    }

    pub async fn sort_by_vram() -> Vec<(usize, f64)> {
        let node_cluster = GLOBAL_CLUSTER.read().await;
        node_cluster.vram_sorting.clone()
    }

    pub async fn filter_node_by_vram(vram_threshold: f64) -> Vec<(usize, f64)> {
        let node_cluster = GLOBAL_CLUSTER.read().await;
        let nodes = node_cluster.vram_sorting.clone();
        let mut filtered_nodes: Vec<_> = nodes.into_iter()
            .filter(|&(_, node_vram)| {
                let vram = if node_vram.is_nan() { 0.0 } else { node_vram };
                vram >= vram_threshold
            })
            .collect();
        filtered_nodes.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        filtered_nodes
    }

    pub async fn size() -> usize {
        let node_cluster = GLOBAL_CLUSTER.read().await;
        node_cluster.size
    }
}
