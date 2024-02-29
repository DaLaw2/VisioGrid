use tokio::select;
use tokio::sync::oneshot;
use crate::utils::logger::{Logger, LogLevel};
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
                    match packet {
                        Ok(packet) => {
                            let packet_type = PacketType::parse_packet_type(&packet.clone_id_byte());
                            let result = match packet_type {
                                PacketType::AlivePacket => self.receiver_tx.alive_packet.send(packet),
                                PacketType::FileBodyPacket => self.receiver_tx.file_body_packet.send(packet),
                                PacketType::FileHeaderPacket => self.receiver_tx.file_header_packet.send(packet),
                                PacketType::StillProcessPacket => self.receiver_tx.still_process_packet.send(packet),
                                PacketType::TaskInfoPacket => self.receiver_tx.task_info_packet.send(packet),
                                _ => {
                                    Logger::add_system_log(LogLevel::WARNING, "Receive Thread: Receive unknown packet.".to_string()).await;
                                    Ok(())
                                },
                            };
                            if result.is_err() {
                                Logger::add_system_log(LogLevel::INFO, "Receive Thread: Unable to submit packet to receiver.".to_string()).await;
                                break;
                            }
                        },
                        Err(_) => {
                            Logger::add_system_log(LogLevel::INFO, "Receive Thread: Agent disconnect.".to_string()).await;
                            break;
                        },
                    }
                },
                _ = &mut self.stop_signal_rx => break,
            }
        }
    }
}
