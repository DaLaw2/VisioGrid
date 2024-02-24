use tokio::select;
use tokio::sync::oneshot;
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::packet::Packet;
use crate::connection::packet::PacketType;
use crate::connection::socket::socket_stream::ReadHalf;
use crate::connection::channel::control_channel_receiver::ReceiverTX;

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
                                PacketType::ConfirmPacket => self.receiver_tx.confirm_packet.send(packet),
                                PacketType::DataChannelPortPacket => self.receiver_tx.data_channel_port_packet.send(packet),
                                _ => {
                                    Logger::append_system_log(LogLevel::WARNING, "Receive Thread: Receive unknown packet.".to_string()).await;
                                    Ok(())
                                },
                            };
                            if result.is_err() {
                                Logger::append_system_log(LogLevel::INFO, "Receive Thread: Unable to submit packet to receiver.".to_string()).await;
                                break;
                            }
                        },
                        Err(_) => {
                            Logger::append_system_log(LogLevel::INFO, "Receive Thread: Agent disconnect.".to_string()).await;
                            break;
                        },
                    }
                },
                _ = &mut self.stop_signal_rx => break,
            }
        }
    }
}
