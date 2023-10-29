use tokio::sync::mpsc;
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::packet::definition::Packet;
use crate::connection::socket::socket_stream::WriteHalf;

pub struct SendThread {
    node_id: usize,
    socket: WriteHalf,
    receiver: mpsc::UnboundedReceiver<Option<Box<dyn Packet + Send>>>
}

impl SendThread {
    pub fn new(node_id: usize, socket: WriteHalf, receiver: mpsc::UnboundedReceiver<Option<Box<dyn Packet + Send>>>) -> Self {
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
                    match self.socket.send_packet(packet).await {
                        Ok(_) => {},
                        Err(_) => Logger::append_node_log(self.node_id, LogLevel::ERROR, "Sender: Failed to send packet.".to_string()).await
                    }
                },
                None => break
            }
        }
    }
}
