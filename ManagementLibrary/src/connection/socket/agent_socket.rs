use crate::connection::socket::socket_stream::SocketStream;
use crate::utils::config::Config;
use crate::utils::logging::*;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;

pub struct AgentSocket {
    listener: TcpListener,
}

impl AgentSocket {
    pub async fn new() -> Self {
        let listener = loop {
            let config = Config::now().await;
            let port = config.agent_listen_port;
            match TcpListener::bind(format!("0.0.0.0:{port}")).await {
                Ok(listener) => break listener,
                Err(err) => {
                    logging_critical!(NetworkEntry::BindPortError(err));
                    sleep(Duration::from_secs(config.bind_retry_duration)).await;
                },
            }
        };
        Self {
            listener,
        }
    }

    pub async fn get_connection(&mut self) -> (SocketStream, SocketAddr) {
        let (stream, address) = loop {
            if let Ok(connection) = self.listener.accept().await {
                break connection;
            }
        };
        (SocketStream::new(stream), address)
    }
}
