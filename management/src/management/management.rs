use crate::connection::socket::agent_socket::AgentSocket;
use crate::management::agent::Agent;
use crate::management::agent_manager::AgentManager;
use crate::management::media_processor::MediaProcessor;
use crate::management::monitor::Monitor;
use crate::utils::config::Config;
use crate::utils::logging::*;
use crate::web::api::{config, default, inference, log, monitor, task};
use actix_web::{App, HttpServer};
use lazy_static::lazy_static;
use std::time::Duration;
use actix_web::web::route;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::time::sleep;
use uuid::Uuid;

lazy_static! {
    static ref MANAGEMENT: RwLock<Management> = RwLock::new(Management::new());
}

pub struct Management {
    terminate: bool,
}

impl Management {
    fn new() -> Self {
        Self {
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
        logging_information!(SystemEntry::Initializing);
        Config::now().await;
        MediaProcessor::run().await;
        Monitor::run().await;
        Self::register_agent().await;
        let http_server = loop {
            let config = Config::now().await;
            let http_server = HttpServer::new(|| {
                let cors = actix_cors::Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600);
                App::new()
                    .wrap(cors)
                    .service(config::initialize())
                    .service(inference::initialize())
                    .service(log::initialize())
                    .service(monitor::initialize())
                    .service(task::initialize())
                    .default_service(route().to(default::default_route))
            })
            .bind(format!("0.0.0.0:{}", config.http_server_bind_port));
            match http_server {
                Ok(http_server) => break http_server,
                Err(err) => {
                    logging_critical!(NetworkEntry::BindPortError(err));
                    sleep(Duration::from_secs(config.bind_retry_duration)).await;
                    continue;
                }
            }
        };
        logging_information!(SystemEntry::WebReady);
        logging_information!(SystemEntry::InitializeComplete);
        logging_information!(SystemEntry::Online);
        if let Err(err) = http_server.run().await {
            logging_emergency!(SystemEntry::WebPanic(err));
        }
    }

    pub async fn terminate() {
        logging_information!(SystemEntry::Terminating);
        Self::instance_mut().await.terminate = true;
        Monitor::terminate().await;
        MediaProcessor::terminate().await;
        logging_information!(SystemEntry::TerminateComplete);
    }

    async fn register_agent() {
        tokio::spawn(async {
            let mut agent_socket = AgentSocket::new().await;
            while !Self::instance().await.terminate {
                let id = Uuid::new_v4();
                let (socket_stream, agent_ip) = agent_socket.get_connection().await;
                let agent = Agent::new(id, socket_stream).await;
                match agent {
                    Ok(agent) => {
                        AgentManager::add_agent(agent).await;
                        logging_information!(SystemEntry::AgentConnect(agent_ip));
                    },
                    Err(entry) => logging_entry!(id, entry),
                }
            }
        });
    }
}
