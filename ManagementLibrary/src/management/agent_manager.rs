use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::stream::{self, StreamExt};
use lazy_static::lazy_static;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::time::sleep;
use uuid::Uuid;

use crate::management::agent::Agent;
use crate::management::utils::agent_state::AgentState;
use crate::management::utils::performance::Performance;
use crate::utils::config::Config;
use crate::utils::logging::*;

lazy_static! {
    static ref AGENT_MANAGER: RwLock<AgentManager> = RwLock::new(AgentManager::new());
}

pub struct AgentManager {
    size: usize,
    agents: HashMap<Uuid, Arc<RwLock<Agent>>>,
    state: HashMap<Uuid, AgentState>,
    performance: HashMap<Uuid, Performance>,
    terminate: bool,
}

impl AgentManager {
    fn new() -> Self {
        Self {
            size: 0_usize,
            agents: HashMap::new(),
            state: HashMap::new(),
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
            Self::refresh_performance().await;
        });
        logging_information!("Agent Manager", "Online now");
    }

    pub async fn terminate() {
        logging_information!("Agent Manager", "Termination in progress");
        Self::instance_mut().await.terminate = true;
        logging_information!("Agent Manager", "Termination complete");
    }

    async fn refresh_performance() {
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
        let mut agent_manager = Self::instance_mut().await;
        if agent_manager.agents.contains_key(&agent_id) {
            logging_error!("Agent Manager", "Agent instance already exists");
            return;
        }
        let agent = Arc::new(RwLock::new(agent));
        agent_manager.agents.insert(agent_id, agent.clone());
        agent_manager.state.insert(agent_id, AgentState::CreateDataChannel);
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

    pub async fn get_agent(agent_id: Uuid) -> Option<Arc<RwLock<Agent>>> {
        let agent_manager = Self::instance().await;
        let agent = agent_manager.agents.get(&agent_id);
        agent.cloned()
    }

    pub async fn store_state(uuid: Uuid, state: AgentState) {
        let mut agent_manager = Self::instance_mut().await;
        match agent_manager.state.get(&uuid) {
            Some(origin_state) => {
                if state >= *origin_state {
                    agent_manager.state.insert(uuid, state);
                }
            },
            None => {
                agent_manager.state.insert(uuid, state);
            },
        }
    }

    pub async fn reset_state(uuid: Uuid) {
        let mut agent_manager = Self::instance_mut().await;
        agent_manager.state.insert(uuid, AgentState::None);
    }

    pub async fn get_state(uuid: Uuid) -> AgentState {
        let agent_manager = Self::instance().await;
        let state = agent_manager.state.get(&uuid).cloned();
        drop(agent_manager);
        match state {
            Some(state) => state,
            None => {
                let mut agent_manager = Self::instance_mut().await;
                agent_manager.state.insert(uuid, AgentState::None);
                AgentState::None
            }
        }
    }

    pub async fn sorted_by_vram() -> Vec<(Uuid, f64)> {
        let agent_manager = Self::instance().await;
        let mut sorted_vram: Vec<(Uuid, f64)> = agent_manager.performance.iter()
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
        filtered_agents.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        filtered_agents
    }

    pub async fn size() -> usize {
        Self::instance().await.size
    }
}
