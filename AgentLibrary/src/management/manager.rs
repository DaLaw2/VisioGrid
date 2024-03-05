use std::sync::Arc;
use tokio::sync::RwLock;
use lazy_static::lazy_static;
use crate::management::agent::Agent;

lazy_static! {
    static ref MANAGER: RwLock<Manager> = RwLock::new(Manager::new());
}

pub struct Manager {
    agent: Option<Arc<RwLock<Agent>>>,
    terminate: bool,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            agent: None,
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
}
