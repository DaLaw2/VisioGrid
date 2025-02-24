use crate::connection::socket::management_socket::ManagementSocket;
use crate::management::agent::Agent;
use crate::management::file_manager::FileManager;
use crate::management::monitor::Monitor;
use crate::management::utils::agent_state::AgentState;
use crate::utils::config::Config;
use crate::utils::logging::*;
use async_ctrlc::CtrlC;
use lazy_static::lazy_static;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::time::sleep;
use common::utils::log_entry::system::SystemEntry;

lazy_static! {
    static ref MANAGEMENT: RwLock<Management> = RwLock::new(Management::new());
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
        logging_information!(SystemEntry::Online);
        match CtrlC::new() {
            Ok(ctrlc) => ctrlc.await,
            Err(err) => logging_emergency!("Unable to create instance", format!("Err: {err}")),
        }
    }

    pub async fn terminate() {
        logging_information!(SystemEntry::Terminating);
        Self::instance_mut().await.terminate = true;
        Monitor::terminate().await;
        FileManager::cleanup().await;
        logging_information!(SystemEntry::TerminateComplete);
    }

    pub async fn hot_reload() {
        let config = Config::now().await;
        while !Self::instance().await.terminate {
            let mut management = Self::instance_mut().await;
            if let Some(agent) = management.agent.clone() {
                if agent.read().await.state == AgentState::Terminate {
                    management.agent = None;
                }
            } else {
                select! {
                    (socket_stream, management_ip) = ManagementSocket::get_connection() => {
                        match Agent::new(socket_stream).await {
                            Ok(agent) => {
                                let agent = Arc::new(RwLock::new(agent));
                                Agent::run(agent.clone()).await;
                                management.agent = Some(agent);
                                logging_information!(SystemEntry::ManagementConnect(management_ip));
                            }
                            Err(entry) => logging_entry!(entry)
                        }
                    }
                    _ = sleep(Duration::from_secs(config.refresh_interval)) => continue
                }
            }
        }
    }
}
