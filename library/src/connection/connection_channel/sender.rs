use uuid::Uuid;
use tokio::sync::mpsc;
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::packet::definition::Packet;
use crate::connection::socket::socket_stream::WriteHalf;

pub struct Sender {
    node_id: Uuid,
    socket: WriteHalf,
    receiver: mpsc::UnboundedReceiver<Option<Box<dyn Packet + Send>>>
}

impl Sender {
    pub fn new(node_id: Uuid, socket: WriteHalf, receiver: mpsc::UnboundedReceiver<Option<Box<dyn Packet + Send>>>) -> Self {
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
                    if self.socket.send_packet(packet).await.is_err() {
                        Logger::append_node_log(self.node_id, LogLevel::ERROR, "Sender: Failed to send packet.".to_string()).await;
                    }
                },
                None => break,
            }
        }
    }
}
