use tokio::time::sleep;
use std::time::Duration;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use crate::utils::logger::*;
use crate::utils::config::Config;
use crate::connection::socket::socket_stream::SocketStream;

pub struct AgentSocket {
    listener: TcpListener,
}

impl AgentSocket {
    pub async fn new() -> Self {
        //unimplemented!("Unable to end normally when failed");
        let listener = loop {
            let config = Config::now().await;
            let port = config.agent_listen_port;
            match TcpListener::bind(format!("127.0.0.1:{port}")).await {
                Ok(listener) => break listener,
                Err(err) => {
                    logging_error!(format!("Agent Socket: Port binding failed.\nReason: {err}"));
                    sleep(Duration::from_secs(config.bind_retry_duration)).await;
                },
            }
        };
        Self {
            listener,
        }
    }

    pub async fn get_connection(&mut self) -> (SocketStream, SocketAddr) {
        //unimplemented!("The loop cannot be ended when there is no connection");
        let (stream, address) = loop {
            if let Ok(connection) = self.listener.accept().await {
                break connection;
            }
        };
        (SocketStream::new(stream), address)
    }
}
