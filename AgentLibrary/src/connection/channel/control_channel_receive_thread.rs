use crate::connection::channel::control_channel_receiver::ReceiverTX;
use crate::connection::packet::Packet;
use crate::connection::packet::PacketType;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::utils::logging::*;
use tokio::select;
use tokio::sync::oneshot;

pub struct ReceiveThread {
    socket_rx: ReadHalf,
    receiver_tx: ReceiverTX,
    stop_signal_rx: oneshot::Receiver<()>,
}

impl ReceiveThread {
    pub fn new(socket_rx: ReadHalf, receiver_tx: ReceiverTX, stop_signal_rx: oneshot::Receiver<()>) -> Self {
        Self {
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
                    if let Ok(packet) = packet {
                        let packet_type = PacketType::parse_packet_type(&packet.clone_id_byte());
                        let result = match packet_type {
                            PacketType::AgentInfoAckPacket => self.receiver_tx.agent_info_ack_packet.send(packet),
                            PacketType::ControlPacket => self.receiver_tx.control_packet.send(packet),
                            PacketType::DataChannelPortPacket => self.receiver_tx.data_channel_port_packet.send(packet),
                            PacketType::PerformanceAckPacket => self.receiver_tx.performance_ack_packet.send(packet),
                            _ => {
                                logging_warning!(NetworkEntry::UnexpectedPacket);
                                Ok(())
                            },
                        };
                        if result.is_err() {
                            logging_information!(NetworkEntry::ChannelClosed);
                            break;
                        }
                    } else {
                        logging_information!(NetworkEntry::ManagementDisconnect);
                        break;
                    }
                },
                _ = &mut self.stop_signal_rx => break,
            }
        }
    }
}
