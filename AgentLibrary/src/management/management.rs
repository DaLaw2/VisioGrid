use std::sync::Arc;
use std::time::Duration;
use async_ctrlc::CtrlC;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use lazy_static::lazy_static;
use tokio::select;
use tokio::time::sleep;
use crate::utils::logging::*;
use crate::connection::socket::management_socket::ManagementSocket;
use crate::management::utils::agent_state::AgentState;
use crate::management::agent::Agent;
use crate::management::monitor::Monitor;
use crate::management::file_manager::FileManager;
use crate::management::utils::agent_state::AgentState;
use crate::connection::socket::management_socket::ManagementSocket;

lazy_static! {
    static ref MANAGER: RwLock<Management> = RwLock::new(Management::new());
}

pub struct Management {
    agent: Option<Arc<RwLock<Agent>>>,
    terminate: bool,
}

impl Management {
    pub fn new() -> Self {
        Self {
            agent: None,
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
        FileManager::initialize().await;
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
        FileManager::cleanup().await;
        logging_information!("Management", "Termination complete");
    }

    pub async fn hot_reload() {
        let config = Config::now().await;
        while !Self::instance().await.terminate {
            let mut management = Self::instance_mut().await;
            if let Some(agent) = management.agent.clone() {
                match agent.read().await.state {
                    AgentState::Terminate => management.agent = None,
                    _ => sleep(Duration::from_millis(config.internal_timestamp)).await,
                }
            } else {
                select! {
                    (socket_stream, management_ip) = ManagementSocket::get_connection() => {
                        match Agent::new(socket_stream).await {
                            Ok(agent) => {
                                let agent = Arc::new(RwLock::new(agent));
                                Agent::run(agent.clone()).await;
                                management.agent = Some(agent);
                                logging_information!("Management", format!("Management {management_ip} is connected"));
                            },
                            Err(entry) => logging_entry!(entry),
                        }
                    },
                    _ = sleep(Duration::from_millis(config.internal_timestamp)) => continue,
                }
            }
        }
    }
}
