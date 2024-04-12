use tokio::select;
use tokio::sync::oneshot;
use crate::utils::logging::*;
use crate::connection::packet::Packet;
use crate::connection::packet::PacketType;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::connection::channel::data_channel_receiver::ReceiverTX;

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
                            PacketType::AlivePacket => self.receiver_tx.alive_packet.send(packet),
                            PacketType::FileBodyPacket => self.receiver_tx.file_body_packet.send(packet),
                            PacketType::FileHeaderPacket => self.receiver_tx.file_header_packet.send(packet),
                            PacketType::FileTransferEndPacket => self.receiver_tx.file_transfer_end_packet.send(packet),
                            PacketType::ResultAcknowledgePacket => self.receiver_tx.result_acknowledge_packet.send(packet),
                            PacketType::StillProcessPacket => self.receiver_tx.still_process_packet.send(packet),
                            PacketType::TaskInfoPacket => self.receiver_tx.task_info_packet.send(packet),
                            _ => {
                                logging_warning!("Receive Thread", "Receive unexpected packet");
                                Ok(())
                            },
                        };
                        if result.is_err() {
                            logging_notice!("Receive Thread", "Channel has been closed");
                            break;
                        }
                    } else {
                        logging_notice!("Receive Thread", "Management side disconnected");
                        break;
                    }
                },
                _ = &mut self.stop_signal_rx => break,
            }
        }
    }
}
