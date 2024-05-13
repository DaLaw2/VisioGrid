use uuid::Uuid;
use std::sync::Arc;
use std::cmp::Ordering;
use std::time::Duration;
use lazy_static::lazy_static;
use std::collections::HashMap;
use chrono::{DateTime, Local};
use futures::stream::{self, StreamExt};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::management::utils::agent_information::AgentInformation;
use crate::utils::logging::*;
use crate::utils::config::Config;
use crate::management::agent::Agent;
use crate::management::utils::performance::Performance;

lazy_static! {
    static ref AGENT_MANAGER: RwLock<AgentManager> = RwLock::new(AgentManager::new());
}

pub struct AgentManager {
    size: usize,
    agents: HashMap<Uuid, Arc<RwLock<Agent>>>,
    information: HashMap<Uuid, AgentInformation>,
    performance: HashMap<Uuid, (Performance, DateTime<Local>)>,
}

impl AgentManager {
    fn new() -> Self {
        Self {
            size: 0_usize,
            agents: HashMap::new(),
            information: HashMap::new(),
            performance: HashMap::new(),
        }
    }

    pub async fn instance() -> RwLockReadGuard<'static, Self> {
        AGENT_MANAGER.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        AGENT_MANAGER.write().await
    }

    pub async fn is_agent_exists(agent_id: Uuid) -> bool {
        let agent_manager = Self::instance().await;
        agent_manager.agents.contains_key(&agent_id)
    }

    pub async fn get_agent(agent_id: Uuid) -> Option<Arc<RwLock<Agent>>> {
        let agent_manager = Self::instance().await;
        let agent = agent_manager.agents.get(&agent_id);
        agent.cloned()
    }

    pub async fn add_agent(agent: Agent) {
        let agent_id = agent.uuid();
        let performance = agent.realtime_usage();
        let mut agent_manager = Self::instance_mut().await;
        if agent_manager.agents.contains_key(&agent_id) {
            logging_error!("Agent Manager", "Agent instance already exists");
            return;
        }
        let agent = Arc::new(RwLock::new(agent));
        agent_manager.agents.insert(agent_id, agent.clone());
        agent_manager.performance.insert(agent_id, (performance, Local::now()));
        agent_manager.size += 1;
        drop(agent_manager);
        Agent::run(agent).await;
    }

    pub async fn remove_agent(agent_id: Uuid) -> Option<Arc<RwLock<Agent>>> {
        let mut agent_manager = Self::instance_mut().await;
        let agent = agent_manager.agents.remove(&agent_id);
        if agent.is_some() {
            agent_manager.size -= 1;
        }
        agent
    }

    pub async fn sorted_by_vram() -> Vec<(Uuid, f64)> {
        let agents: Vec<Uuid> = Self::instance().await.agents.keys().cloned().collect();
        let mut agents: Vec<(Uuid, f64)> = stream::iter(agents)
            .then(|agent_id| async move {
                let performance = Self::get_agent_performance(agent_id).await;
                match performance {
                    Some(performance) => (agent_id, performance.vram),
                    None => (agent_id, 0.0),
                }
            })
            .collect()
            .await;
        agents.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        agents
    }

    pub async fn filter_agent_by_vram(vram_threshold: f64) -> Vec<(Uuid, f64)> {
        let agents = Self::sorted_by_vram().await;
        let mut filtered_agents: Vec<_> = agents.into_iter()
            .filter(|&(_, agent_vram)| {
                let vram = if agent_vram.is_nan() { 0.0 } else { agent_vram };
                vram >= vram_threshold
            })
            .collect();
        filtered_agents.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        filtered_agents
    }

    pub async fn get_agent_information(agent_id: Uuid) -> Option<AgentInformation> {
        let agent_manager = Self::instance().await;
        let information = agent_manager.information.get(&agent_id);
        information.cloned()
    }

    pub async fn get_agent_performance(agent_id: Uuid) -> Option<Performance> {
        let config = Config::now().await;
        let performance = Self::instance().await.performance.get(&agent_id).cloned()?;
        if performance.1 < Local::now() - Duration::from_secs(config.refresh_interval) {
            let performance = Self::refresh_performance(agent_id).await?;
            Some(performance)
        } else {
            Some(performance.0)
        }
    }

    async fn refresh_performance(uuid: Uuid) -> Option<Performance> {
        let agent = AgentManager::get_agent(uuid).await?;
        let performance = agent.read().await.realtime_usage();
        let mut agent_manager = Self::instance_mut().await;
        agent_manager.performance.insert(uuid, (performance, Local::now()));
        Some(performance)
    }

    pub async fn size() -> usize {
        Self::instance().await.size
    }
}
