use uuid::Uuid;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Arc;
use chrono::{DateTime, Local};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::management::agent::Agent;
use crate::management::agent_manager::AgentManager;
use crate::management::utils::agent_information::AgentInformation;
use crate::management::utils::performance::Performance;

lazy_static! {
    static ref MONITOR: RwLock<Monitor> = RwLock::new(Monitor::new());
}

struct Monitor {
    information: HashMap<Uuid, AgentInformation>,
    performance: HashMap<Uuid, (Performance, DateTime<Local>)>,
}

impl Monitor {
    pub fn new() -> Monitor {
        Monitor {
            information: HashMap::new(),
            performance: HashMap::new(),
        }
    }

    pub async fn instance() -> RwLockReadGuard<'static, Self> {
        MONITOR.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        MONITOR.write().await
    }

    pub async fn register(&mut self, agent: &mut Agent) {
        let uuid = agent.uuid();
        let information = agent.agent_information();
        let performance = agent.realtime_usage();

    }
}
