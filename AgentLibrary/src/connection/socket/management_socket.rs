use crate::connection::socket::socket_stream::SocketStream;
use crate::utils::config::Config;
use std::net::SocketAddr;
use tokio::net::TcpStream;

pub struct ManagementSocket;

impl ManagementSocket {
    pub async fn get_connection() -> (SocketStream, SocketAddr) {
        let config = Config::now().await;
        let address = format!("{}:{}", config.management_address, config.management_port);
        loop {
            if let Ok(tcp_stream) = TcpStream::connect(&address).await {
                if let Ok(socket_address) = tcp_stream.peer_addr() {
                    let socket_stream = SocketStream::new(tcp_stream);
                    break (socket_stream, socket_address);
                }
            }
        }
    }
}
