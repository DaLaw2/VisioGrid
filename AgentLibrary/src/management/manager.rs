use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use lazy_static::lazy_static;
use crate::management::utils::agent_state::AgentState;
use crate::management::agent::Agent;

lazy_static! {
    static ref MANAGER: RwLock<Manager> = RwLock::new(Manager::new());
}

pub struct Manager {
    agent: Option<Arc<RwLock<Agent>>>,
    state: Option<AgentState>,
    terminate: bool,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            agent: None,
            state: None,
            terminate: false,
        }
    }

    pub async fn instance() -> RwLockReadGuard<'static, Self> {
        MANAGER.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        MANAGER.write().await
    }

    pub async fn run() {

    }

    fn initialize() {

    }

    pub async fn terminate() {

    }

    fn cleanup() {

    }

    pub async fn store_state(state: AgentState) {
        let mut manager = Self::instance_mut().await;
        if let Some(origin_state) = manager.state {
            if origin_state != AgentState::Terminate {
                manager.state = Some(state);
            }
        } else {
            manager.state = Some(state)
        }
    }

    pub async fn get_state() -> AgentState {
        let manager = Self::instance().await;
        manager.state.unwrap_or_else(|| AgentState::None)
    }
}
