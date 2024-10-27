use crate::connection::channel::data_channel_receiver::ReceiverTX;
use crate::connection::packet::Packet;
use crate::connection::packet::PacketType;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::utils::logging::*;
use tokio::select;
use tokio::sync::oneshot;
use uuid::Uuid;

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
                                PacketType::AliveAckPacket => self.receiver_tx.alive_ack_packet.send(packet),
                                PacketType::FileBodyPacket => self.receiver_tx.file_body_packet.send(packet),
                                PacketType::FileHeaderAckPacket => self.receiver_tx.file_header_ack_packet.send(packet),
                                PacketType::FileHeaderPacket => self.receiver_tx.file_header_packet.send(packet),
                                PacketType::FileTransferEndPacket => self.receiver_tx.file_transfer_end_packet.send(packet),
                                PacketType::FileTransferResultPacket => self.receiver_tx.file_transfer_result_packet.send(packet),
                                PacketType::StillProcessAckPacket => self.receiver_tx.still_process_ack_packet.send(packet),
                                PacketType::TaskInfoAckPacket => self.receiver_tx.task_info_ack_packet.send(packet),
                                PacketType::TaskResultPacket => self.receiver_tx.task_result_packet.send(packet),
                                _ => {
                                    logging_warning!(self.agent_id, NetworkEntry::UnexpectedPacket, "");
                                    Ok(())
                                },
                            };
                            if result.is_err() {
                                logging_information!(self.agent_id, NetworkEntry::ChannelClosed, "");
                                return;
                            }
                        },
                        Err(err) => {
                            logging_information!(self.agent_id, NetworkEntry::AgentDisconnect, format!("Err: {err}"));
                            return;
                        },
                    }
                },
                _ = &mut self.stop_signal_rx => return,
            }
        }
    }
}
