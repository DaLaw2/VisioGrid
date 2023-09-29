use tokio::sync::mpsc;
use crate::logger::logger::{Logger, LogLevel};
use crate::connection::packet::definition::Packet;
use crate::connection::socket::node_socket::NodeSocket;

pub struct SendThread {
    node_id: usize,
    socket: NodeSocket,
    receiver: mpsc::UnboundedReceiver<Option<Box<dyn Packet + Send>>>
}

impl SendThread {
    pub fn new(node_id: usize, socket: NodeSocket, receiver: mpsc::UnboundedReceiver<Option<Box<dyn Packet + Send>>>) -> Self {
        Self {
            node_id,
            socket,
            receiver,
        }
    }

    pub async fn run(&mut self) {
        while let Some(packet) = self.receiver.recv().await {
            match packet {
                Some(packet) => {
                    match self.socket.send_packet(&*packet).await {
                        Ok(_) => Logger::instance().append_node_log(self.node_id, LogLevel::INFO, format!("Packet sent: {}", packet.to_string())),
                        Err(_) => {
                            Logger::instance().append_node_log(self.node_id, LogLevel::ERROR, "Fail send packet.".to_string());
                            Logger::instance().append_system_log(LogLevel::ERROR, format!("Node {}: Fail send paket.", self.node_id));
                        }
                    }
                },
                None => break
            }
        }
    }
}
