use tokio::net::TcpListener;
use std::collections::BTreeSet;
use std::time::Duration;
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::socket::socket_stream::SocketStream;

pub struct NodeSocket {
    listener: TcpListener,
    id_generator: SocketIDGenerator
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
            listener,
            id_generator: SocketIDGenerator::new()
        }
    }

    pub async fn get_connection(&mut self) -> SocketStream {
        let (stream, _) = loop {
            if let Ok(connection) = self.listener.accept().await {
                break connection;
            }
        };
        SocketStream::new(self.id_generator.allocate_id(), stream)
    }
}

struct SocketIDGenerator {
    available: BTreeSet<usize>,
    next: usize,
}

impl SocketIDGenerator {
    fn new() -> Self {
        SocketIDGenerator {
            available: BTreeSet::new(),
            next: 0,
        }
    }

    fn allocate_id(&mut self) -> usize {
        if let Some(&first) = self.available.iter().next() {
            self.available.remove(&first);
            first
        } else {
            let current = self.next;
            self.next += 1;
            current
        }
    }

    fn free_id(&mut self, port: usize) {
        if port == self.next - 1 {
            self.next -= 1;
        } else {
            self.available.insert(port);
        }
    }
}
