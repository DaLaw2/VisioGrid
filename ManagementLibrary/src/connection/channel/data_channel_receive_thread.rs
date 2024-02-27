use uuid::Uuid;
use tokio::select;
use tokio::sync::oneshot;
use crate::connection::packet::Packet;
use crate::connection::packet::PacketType;
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::socket::socket_stream::ReadHalf;
use crate::connection::channel::data_channel_receiver::ReceiverTX;

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
                    if let Ok(packet) = packet {
                        let packet_type = PacketType::parse_packet_type(&packet.clone_id_byte());
                        let result = match packet_type {
                            PacketType::AliveReplyPacket => self.receiver_tx.alive_reply_packet.send(packet),
                            PacketType::FileTransferReplyPacket => self.receiver_tx.file_transfer_reply_packet.send(packet),
                            PacketType::ResultPacket => self.receiver_tx.result_packet.send(packet),
                            PacketType::StillProcessReplyPacket => self.receiver_tx.still_process_reply_packet.send(packet),
                            PacketType::TaskInfoReplyPacket => self.receiver_tx.task_info_reply_packet.send(packet),
                            _ => {
                                Logger::add_agent_log(self.agent_id, LogLevel::WARNING, "Receive Thread: Receive unknown packet.".to_string()).await;
                                Ok(())
                            },
                        };
                        if result.is_err() {
                            Logger::add_agent_log(self.agent_id, LogLevel::INFO, "Receive Thread: Unable to submit packet to receiver.".to_string()).await;
                            break;
                        }
                    } else {
                        Logger::add_agent_log(self.agent_id, LogLevel::INFO, "Receive Thread: Agent disconnect.".to_string()).await;
                        break;
                    }
                },
                _ = &mut self.stop_signal_rx => break,
            }
        }
    }
}
