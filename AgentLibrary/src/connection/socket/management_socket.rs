use std::net::SocketAddr;
use tokio::net::TcpStream;
use crate::utils::config::Config;
use crate::connection::socket::socket_stream::SocketStream;

pub struct ManagementSocket;

impl ManagementSocket {
    pub async fn get_connection() -> (SocketStream, SocketAddr) {
        let config = Config::now().await;
        loop {
            if let Ok(tcp_stream) = TcpStream::connect(&config.management_address).await {
                if let Ok(socket_address) = tcp_stream.peer_addr() {
                    let socket_stream = SocketStream::new(tcp_stream);
                    break (socket_stream, socket_address);
                }
            }
        }
    }
}
