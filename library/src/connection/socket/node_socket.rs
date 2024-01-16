use std::time::Duration;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use crate::utils::config::Config;
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::socket::socket_stream::SocketStream;

pub struct NodeSocket {
    listener: TcpListener
}

impl NodeSocket {
    pub async fn new() -> Self {
        let config = Config::now().await;
        let listener = loop {
            match TcpListener::bind(format!("127.0.0.1:{}", config.node_listen_port)).await {
                Ok(listener) => {
                    Logger::append_system_log(LogLevel::INFO, format!("Node Socket: Port binding successful.\nOn port {}.", config.node_listen_port)).await;
                    break listener;
                },
                Err(_) => {
                    Logger::append_system_log(LogLevel::ERROR, format!("Node Socket: Port binding failed.\nTry after {}s.", config.bind_retry_duration)).await;
                    tokio::time::sleep(Duration::from_secs(config.bind_retry_duration)).await;
                }
            }
        };
        Self {
            listener
        }
    }

    pub async fn get_connection(&mut self) -> (SocketStream, SocketAddr) {
        let (stream, address) = loop {
            if let Ok(connection) = self.listener.accept().await {
                break connection;
            }
        };
        (SocketStream::new(stream, address.clone()), address)
    }
}
