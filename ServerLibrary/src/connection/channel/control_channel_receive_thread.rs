use uuid::Uuid;
use tokio::select;
use tokio::sync::oneshot;
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::packet::Packet;
use crate::connection::packet::PacketType;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::connection::channel::control_channel_receiver::ReceiverTX;

pub struct ReceiveThread {
    node_id: Uuid,
    socket_rx: ReadHalf,
    receiver_tx: ReceiverTX,
    stop_signal_rx: oneshot::Receiver<()>,
}

impl ReceiveThread {
    pub fn new(node_id: Uuid, socket_rx: ReadHalf, receiver_tx: ReceiverTX, stop_signal_rx: oneshot::Receiver<()>) -> Self {
        Self {
            node_id,
            socket_rx,
            receiver_tx,
            stop_signal_rx,
        }
    }

    pub async fn run(&mut self) {
        loop {
            select! {
                biased;
                packet = self.socket_rx.receive_packet() => {
                    match packet {
                        Ok(packet) => {
                            let packet_type = PacketType::parse_packet_type(&packet.clone_id_byte());
                            let result = match packet_type {
                                PacketType::NodeInformationPacket => self.receiver_tx.node_information_packet.send(packet),
                                PacketType::PerformancePacket => self.receiver_tx.performance_packet.send(packet),
                                _ => {
                                    Logger::append_node_log(self.node_id, LogLevel::WARNING, "Receive Thread: Receive unknown packet.".to_string()).await;
                                    Ok(())
                                },
                            };
                            if result.is_err() {
                                Logger::append_node_log(self.node_id, LogLevel::INFO, "Receive Thread: Unable to submit packet to receiver.".to_string()).await;
                                break;
                            }
                        },
                        Err(_) => {
                            Logger::append_node_log(self.node_id, LogLevel::INFO, "Receive Thread: Client disconnect.".to_string()).await;
                            break;
                        },
                    }
                },
                _ = &mut self.stop_signal_rx => break,
            }
        }
    }
}
