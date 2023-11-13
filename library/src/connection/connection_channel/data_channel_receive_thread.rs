use tokio::sync::oneshot;
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::socket::socket_stream::ReadHalf;
use crate::connection::connection_channel::data_packet_channel::PacketSender;

pub struct ReceiveThread {
    node_id: usize,
    socket: ReadHalf,
    stop_signal: oneshot::Receiver<()>,
    data_packet_channel: PacketSender,
}

impl ReceiveThread {
    pub fn new(node_id: usize, socket: ReadHalf, stop_signal: oneshot::Receiver<()>, data_packet_channel: PacketSender) -> Self {
        Self {
            node_id,
            socket,
            stop_signal,
            data_packet_channel,
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                result = self.socket.receive_packet() => {

                }
                _ = &mut self.stop_signal => break
            }
        }
    }
}
