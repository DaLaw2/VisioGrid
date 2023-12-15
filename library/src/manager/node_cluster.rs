use uuid::Uuid;
use std::sync::Arc;
use tokio::time::sleep;
use std::time::Duration;
use lazy_static::lazy_static;
use std::collections::HashMap;
use futures::stream::{self, StreamExt};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::manager::node::Node;
use crate::utils::config::Config;

lazy_static! {
    static ref GLOBAL_CLUSTER: RwLock<NodeCluster> = RwLock::new(NodeCluster::new());
}

pub struct NodeCluster {
    size: usize,
    nodes: HashMap<Uuid, Arc<RwLock<Node>>>,
    vram_sorting: Vec<(Uuid, f64)>,
}

impl NodeCluster {
    fn new() -> Self {
        Self {
            size: 0_usize,
            nodes: HashMap::new(),
            vram_sorting: Vec::new(),
        }
    }

    pub async fn instance() -> RwLockReadGuard<'static, Self> {
        GLOBAL_CLUSTER.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        GLOBAL_CLUSTER.write().await
    }

    pub async fn run() {
        tokio::spawn(async {
            let config = Config::now().await;
            loop {
                {
                    let mut node_cluster = GLOBAL_CLUSTER.write().await;
                    let mut vram: Vec<(Uuid, f64)> = stream::iter(&node_cluster.nodes)
                        .then(|(&key, node)| async move {
                            (key, node.read().await.idle_unused().vram)
                        }).collect().await;
                    vram.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                    node_cluster.vram_sorting = vram;
                }
                sleep(Duration::from_millis(config.internal_timestamp as u64)).await;
            }
        });
    }

    pub async fn add_node(node: Node) {
        let mut node_cluster = GLOBAL_CLUSTER.write().await;
        let node_id = node.uuid();
        if node_cluster.nodes.contains_key(&node_id) {
            return;
        }
        let node = Arc::new(RwLock::new(node));
        node_cluster.nodes.insert(node_id, node.clone());
        node_cluster.size += 1;
        Node::run(node).await;
    }

    pub async fn remove_node(node_id: Uuid) -> Option<Arc<RwLock<Node>>> {
        let mut node_cluster = GLOBAL_CLUSTER.write().await;
        let node = node_cluster.nodes.remove(&node_id);
        if node.is_some() {
            node_cluster.size -= 1;
        }
        node
    }

    pub async fn get_node(node_id: Uuid) -> Option<Arc<RwLock<Node>>> {
        let node_cluster = GLOBAL_CLUSTER.read().await;
        let node = node_cluster.nodes.get(&node_id);
        match node {
            Some(node) => Some(node.clone()),
            None => None
        }
    }

    pub async fn sorted_by_vram() -> Vec<(Uuid, f64)> {
        let node_cluster = GLOBAL_CLUSTER.read().await;
        node_cluster.vram_sorting.clone()
    }

    pub async fn filter_node_by_vram(vram_threshold: f64) -> Vec<(Uuid, f64)> {
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
