use uuid::Uuid;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::packet::definition::Packet;
use crate::connection::socket::socket_stream::WriteHalf;

type SenderRX = mpsc::UnboundedReceiver<Box<dyn Packet + Send>>;

pub struct SendThread {
    node_id: Uuid,
    socket_tx: WriteHalf,
    sender_rx: SenderRX,
    stop_signal_rx: oneshot::Receiver<()>,
}

impl SendThread {
    pub fn new(node_id: Uuid, socket_tx: WriteHalf, sender_rx: SenderRX, stop_signal_rx: oneshot::Receiver<()>) -> Self {
        Self {
            node_id,
            socket_tx,
            sender_rx,
            stop_signal_rx,
        }
    }

    pub async fn run(&mut self) {
        loop {
            select! {
                biased;
                reply = self.sender_rx.recv() => {
                    match reply {
                        Some(packet) => {
                            if self.socket_tx.send_packet(packet).await.is_err() {
                                Logger::append_node_log(self.node_id, LogLevel::ERROR, "Send Thread: Failed to send packet.".to_string()).await;
                            }
                        },
                        None => break,
                    }
                },
                _ = &mut self.stop_signal_rx => break,
            }
        }
    }
}
