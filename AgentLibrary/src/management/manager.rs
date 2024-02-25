use std::sync::Arc;
use tokio::sync::RwLock;
use lazy_static::lazy_static;
use crate::management::utils::performance::Performance;
use crate::management::agent::Agent;
use crate::management::utils::agent_information::AgentInformation;

lazy_static! {
    static ref MANAGER: RwLock<Manager> = RwLock::new(Manager::new());
}

pub struct Manager {
    agent: Option<Arc<RwLock<Agent>>>,
    information: AgentInformation,
    terminate: bool,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            agent: None,
            terminate: false,
        }
    }

    pub fn run() {

    }

    fn initialize() {

    }

    pub fn terminate() {

    }

    fn cleanup() {

    }

    pub async fn get_performance() -> Performance {

    }

    pub async fn get_information() -> AgentInformation {

    }
}
