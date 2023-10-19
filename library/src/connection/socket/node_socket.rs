use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use crate::utils::config::Config;
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::socket::socket_stream::SocketStream;

pub struct NodeSocket {
    listener: TcpListener
}

impl NodeSocket {
    pub async fn new() -> Self {
        let port = Config::instance().await.node_listen_port;
        let bind_retry_duration = Config::instance().await.bind_retry_duration;
        let listener = loop {
            match TcpListener::bind(format!("127.0.0.1:{}", port)).await {
                Ok(listener) => {
                    Logger::instance().await.append_system_log(LogLevel::INFO, format!("Port bind successful.\nOn port {}.", port));
                    break listener;
                },
                Err(_) => {
                    Logger::instance().await.append_system_log(LogLevel::ERROR, format!("Port bind failed.\nTry after {}s.", bind_retry_duration));
                    tokio::time::sleep(Duration::from_secs(bind_retry_duration as u64)).await;
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
