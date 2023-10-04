use std::time::Duration;
use tokio::net::TcpListener;
use crate::utils::id_manager::IDManager;
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::socket::socket_stream::SocketStream;

pub struct NodeSocket {
    id: IDManager,
    listener: TcpListener
}

impl NodeSocket {
    pub async fn new(port: usize) -> Self {
        let listener = loop {
            match TcpListener::bind(format!("127.0.0.1:{}", port)).await {
                Ok(listener) => {
                    Logger::instance().append_system_log(LogLevel::INFO, format!("Port bind successful.\nOn port {}.", port));
                    break listener;
                },
                Err(_) => {
                    Logger::instance().append_system_log(LogLevel::ERROR, "Port bind failed.\nTry after 30s.".to_string());
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            }
        };
        Self {
            id: IDManager::new(),
            listener
        }
    }

    pub async fn get_connection(&mut self) -> SocketStream {
        let (stream, _) = loop {
            if let Ok(connection) = self.listener.accept().await {
                break connection;
            }
        };
        SocketStream::new(self.id.allocate_id(), stream)
    }
}
