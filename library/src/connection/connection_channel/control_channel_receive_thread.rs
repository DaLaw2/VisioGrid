use tokio::sync::oneshot;
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::packet::definition::Packet;
use crate::connection::packet::definition::PacketType;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::connection::connection_channel::control_packet_channel::PacketSender;

pub struct ReceiveThread {
    node_id: usize,
    socket: ReadHalf,
    stop_signal: oneshot::Receiver<()>,
    control_packet_channel: PacketSender,
}

impl ReceiveThread {
    pub fn new(node_id: usize, socket: ReadHalf, stop_signal: oneshot::Receiver<()>, control_packet_channel: PacketSender) -> Self {
        Self {
            node_id,
            socket,
            stop_signal,
            control_packet_channel,
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                packet = self.socket.receive_packet() => {
                    match packet {
                        Ok(packet) => {
                            let packet_type = PacketType::parse_packet_type(&packet.clone_id_byte());
                            let result = match packet_type {
                                PacketType::ControlReplyPacket => self.control_packet_channel.control_reply_packet.send(packet),
                                PacketType::PerformancePacket => self.control_packet_channel.performance_packet.send(packet),
                                _ => {
                                    Logger::append_node_log(self.node_id, LogLevel::WARNING, "Control Channel Receiver: Receive unknown packet.".to_string()).await;
                                    Ok(())
                                }
                            };
                            if result.is_err() {
                                Logger::append_node_log(self.node_id, LogLevel::INFO, "Control Channel Receiver: Client disconnect.".to_string()).await;
                                break;
                            }
                        },
                        Err(_) => {
                            Logger::append_node_log(self.node_id, LogLevel::INFO, "Control Channel Receiver: Client disconnect.".to_string()).await;
                            break;
                        }
                    }
                }
                _ = &mut self.stop_signal => break
            }
        }
    }
}
