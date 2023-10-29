use tokio::sync::{mpsc, oneshot};
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::socket::socket_stream::ReadHalf;
use crate::connection::packet::base_packet::BasePacket;

pub struct ReceiveThread {
    node_id: usize,
    socket: ReadHalf,
    sender: mpsc::UnboundedSender<BasePacket>,
    stop_signal: oneshot::Receiver<()>
}

impl ReceiveThread {
    pub fn new(node_id: usize, socket: ReadHalf, sender: mpsc::UnboundedSender<BasePacket>, stop_signal: oneshot::Receiver<()>) -> Self {
        Self {
            node_id,
            socket,
            sender,
            stop_signal
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                result = self.socket.receive_packet() => {
                    match result {
                        Ok(packet) => {
                            if self.sender.send(packet).is_err() {
                                Logger::append_node_log(self.node_id, LogLevel::INFO, format!("Receiver: Client disconnect.")).await;
                                break;
                            }
                        },
                        Err(_) => {
                            Logger::append_node_log(self.node_id, LogLevel::INFO, format!("Receiver: Client disconnect.")).await;
                            break;
                        }
                    }
                }
                _ = &mut self.stop_signal => break
            }
        }
    }
}
