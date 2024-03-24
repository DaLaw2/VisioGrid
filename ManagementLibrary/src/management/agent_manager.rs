use std::cmp::Ordering;
use uuid::Uuid;
use std::sync::Arc;
use tokio::time::sleep;
use std::time::Duration;
use lazy_static::lazy_static;
use std::collections::HashMap;
use futures::stream::{self, StreamExt};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::management::utils::performance::Performance;
use crate::utils::config::Config;
use crate::management::agent::Agent;
use crate::utils::logger::{Logger, LogLevel};

lazy_static! {
    static ref AGENT_MANAGER: RwLock<AgentManager> = RwLock::new(AgentManager::new());
}

pub struct AgentManager {
    size: usize,
    agents: HashMap<Uuid, Arc<RwLock<Agent>>>,
    performance: HashMap<Uuid, Performance>,
    terminate: bool,
}

impl AgentManager {
    fn new() -> Self {
        Self {
            size: 0_usize,
            agents: HashMap::new(),
            performance: HashMap::new(),
            terminate: false,
        }
    }

    pub async fn instance() -> RwLockReadGuard<'static, Self> {
        AGENT_MANAGER.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        AGENT_MANAGER.write().await
    }

    pub async fn run() {
        tokio::spawn(async {
            Self::update_performance().await;
        });
        Logger::add_system_log(LogLevel::INFO, "Agent Manager: Online.".to_string()).await;
    }

    pub async fn terminate() {
        Logger::add_system_log(LogLevel::INFO, "Agent Manager: Terminating.".to_string()).await;
        Self::instance_mut().await.terminate = true;
        Logger::add_system_log(LogLevel::INFO, "Agent Manager: Termination complete.".to_string()).await;
    }

    async fn update_performance() {
        let config = Config::now().await;
        while !Self::instance().await.terminate {
            let mut agent_manager = Self::instance_mut().await;
            let performance: HashMap<Uuid, Performance> = stream::iter(&agent_manager.agents)
                .then(|(&key, agent)| async move {
                    (key, agent.read().await.idle_unused())
                }).collect().await;
            agent_manager.performance = performance;
            drop(agent_manager);
            sleep(Duration::from_millis(config.internal_timestamp)).await;
        }
    }

    pub async fn add_agent(agent: Agent) {
        let agent_id = agent.uuid();
        let mut agent_cluster = Self::instance_mut().await;
        if agent_cluster.agents.contains_key(&agent_id) {
            Logger::add_system_log(LogLevel::ERROR, "Agent Manager: Agent is already inside.".to_string()).await;
            return;
        }
        let agent = Arc::new(RwLock::new(agent));
        agent_cluster.agents.insert(agent_id, agent.clone());
        agent_cluster.size += 1;
        Agent::run(agent).await;
    }

    pub async fn remove_agent(agent_id: Uuid) -> Option<Arc<RwLock<Agent>>> {
        let mut agent_cluster = Self::instance_mut().await;
        let agent = agent_cluster.agents.remove(&agent_id);
        if agent.is_some() {
            agent_cluster.size -= 1;
        }
        agent
    }

    pub async fn get_agent(agent_id: Uuid) -> Option<Arc<RwLock<Agent>>> {
        let agent_cluster = Self::instance().await;
        let agent = agent_cluster.agents.get(&agent_id);
        match agent {
            Some(agent) => Some(agent.clone()),
            None => None
        }
    }

    pub async fn sorted_by_vram() -> Vec<(Uuid, f64)> {
        let agent_cluster = Self::instance().await;
        let mut sorted_vram: Vec<(Uuid, f64)> = agent_cluster.performance.iter()
            .map(|(uuid, performance)| (uuid.clone(), performance.vram))
            .collect();
        sorted_vram.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        sorted_vram
    }

    pub async fn filter_agent_by_vram(vram_threshold: f64) -> Vec<(Uuid, f64)> {
        let agents = Self::sorted_by_vram().await;
        let mut filtered_agents: Vec<_> = agents.into_iter()
            .filter(|&(_, agent_vram)| {
                let vram = if agent_vram.is_nan() { 0.0 } else { agent_vram };
                vram >= vram_threshold
            })
            .collect();
        filtered_agents.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        filtered_agents
    }

    pub async fn size() -> usize {
        Self::instance().await.size
    }
}
