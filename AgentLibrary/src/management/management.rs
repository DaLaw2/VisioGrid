use tokio::select;
use std::sync::Arc;
use tokio::time::sleep;
use async_ctrlc::CtrlC;
use std::time::Duration;
use lazy_static::lazy_static;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::utils::logging::*;
use crate::utils::config::Config;
use crate::management::agent::Agent;
use crate::management::file_manager::FileManager;
use crate::management::utils::agent_state::AgentState;
use crate::connection::socket::management_socket::ManagementSocket;
use crate::management::monitor::Monitor;

lazy_static! {
    static ref MANAGEMENT: RwLock<Management> = RwLock::new(Management::new());
}

pub struct Management {
    agent: Option<Arc<RwLock<Agent>>>,
    state: Option<AgentState>,
    terminate: bool,
}

impl Management {
    pub fn new() -> Self {
        Self {
            agent: None,
            state: None,
            terminate: false,
        }
    }

    pub async fn instance() -> RwLockReadGuard<'static, Self> {
        MANAGEMENT.read().await
    }

    pub async fn instance_mut() -> RwLockWriteGuard<'static, Self> {
        MANAGEMENT.write().await
    }

    pub async fn run() {
        FileManager::initialize().await;
        Monitor::run().await;
        tokio::spawn(async move {
            Self::hot_reload().await;
        });
        logging_information!("Management", "Online now");
        match CtrlC::new() {
            Ok(ctrlc) => ctrlc.await,
            Err(err) => logging_emergency!("Management", "Unable to create instance", format!("Err: {err}")),
        }
    }

    pub async fn terminate() {
        logging_information!("Management", "Termination in process");
        Self::instance_mut().await.terminate = true;
        Monitor::terminate().await;
        FileManager::cleanup().await;
        logging_information!("Management", "Termination complete");
    }

    pub async fn hot_reload() {
        let config = Config::now().await;
        while !Self::instance().await.terminate {
            let mut management = Self::instance_mut().await;
            if management.agent.is_some() {
                match management.state {
                    Some(AgentState::Terminate) => {
                        management.agent = None;
                        management.state = Some(AgentState::None);
                    }
                    _ => sleep(Duration::from_millis(config.internal_timestamp)).await,
                }
            } else {
                select! {
                    (socket_stream, _) = ManagementSocket::get_connection() => {
                        match Agent::new(socket_stream).await {
                            Ok(agent) => {
                                let agent = Arc::new(RwLock::new(agent));
                                Agent::run(agent.clone()).await;
                                management.agent = Some(agent);
                            },
                            Err(entry) => logging_entry!(entry),
                        }
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            }
        }
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
