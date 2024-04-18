use uuid::Uuid;
use tokio::select;
use tokio::sync::oneshot;
use crate::utils::logging::*;
use crate::connection::packet::Packet;
use crate::connection::packet::PacketType;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::connection::channel::control_channel_receiver::ReceiverTX;

pub struct ReceiveThread {
    agent_id: Uuid,
    socket_rx: ReadHalf,
    receiver_tx: ReceiverTX,
    stop_signal_rx: oneshot::Receiver<()>,
}

impl ReceiveThread {
    pub fn new(agent_id: Uuid, socket_rx: ReadHalf, receiver_tx: ReceiverTX, stop_signal_rx: oneshot::Receiver<()>) -> Self {
        Self {
            agent_id,
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
                                PacketType::AgentInformationPacket => self.receiver_tx.agent_information_packet.send(packet),
                                PacketType::ControlAcknowledgePacket => self.receiver_tx.control_acknowledge_packet.send(packet),
                                PacketType::PerformancePacket => self.receiver_tx.performance_packet.send(packet),
                                _ => {
                                    logging_warning!(self.agent_id, "Receive Thread", "Receive unexpected packet", "");
                                    Ok(())
                                },
                            };
                            if result.is_err() {
                                logging_notice!(self.agent_id, "Receive Thread", "Channel has been closed", "");
                                return;
                            }
                        },
                        Err(err) => {
                            logging_notice!(self.agent_id, "Receive Thread", "Agent side disconnected", format!("Err: {err}"));
                            break;
                        },
                    }
                },
                _ = &mut self.stop_signal_rx => break,
            }
        }
    }
}
