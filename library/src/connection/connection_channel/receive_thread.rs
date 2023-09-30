use tokio::sync::mpsc;
use crate::logger::logger::{Logger, LogLevel};
use crate::connection::socket::node_socket::ReadHalf;
use crate::connection::packet::base_packet::BasePacket;

pub struct ReceiveThread {
    node_id: usize,
    socket: ReadHalf,
    sender: mpsc::UnboundedSender<BasePacket>,
}

impl ReceiveThread {
    pub fn new(node_id: usize, socket: ReadHalf, sender: mpsc::UnboundedSender<BasePacket>) -> Self {
        Self {
            node_id,
            socket,
            sender,
        }
    }

    pub async fn run(&mut self) {
        loop {
            match self.socket.receive_packet().await {
                Ok(packet) => {
                    if self.sender.send(packet).is_err() {
                        Logger::instance().append_node_log(self.node_id, LogLevel::INFO, format!("Client disconnect."));
                        Logger::instance().append_global_log(LogLevel::INFO, format!("Node {}: Client disconnect.", self.node_id));
                        break;
                    }
                },
                Err(_) => {
                    Logger::instance().append_node_log(self.node_id, LogLevel::INFO, format!("Client disconnect."));
                    Logger::instance().append_global_log(LogLevel::INFO, format!("Node {}: Client disconnect.", self.node_id));
                    break;
                }
            }
        }
    }
}