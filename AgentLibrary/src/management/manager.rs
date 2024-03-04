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
            information: Self::get_information(),
            terminate: false,
        }
    }

    pub async fn run() {

    }

    fn initialize() {

    }

    pub async fn terminate() {

    }

    fn cleanup() {

    }

    pub async fn get_performance() -> Performance {
        Performance {
            cpu: 0.0,
            ram: 0.0,
            gpu: 0.0,
            vram: 0.0,
        }
    }

    pub fn get_information() -> AgentInformation {
        AgentInformation {
            host_name: "".to_string(),
            system_name: "".to_string(),
            cpu: "".to_string(),
            cores: 0,
            ram: 0,
            gpu: "".to_string(),
            vram: 0,
        }
    }
}
