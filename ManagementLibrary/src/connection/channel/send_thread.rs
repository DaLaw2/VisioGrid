use uuid::Uuid;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use crate::utils::logging::*;
use crate::connection::packet::Packet;
use crate::connection::socket::socket_stream::WriteHalf;

type SenderRX = mpsc::UnboundedReceiver<Box<dyn Packet + Send>>;

pub struct SendThread {
    agent_id: Uuid,
    socket_tx: WriteHalf,
    sender_rx: SenderRX,
    stop_signal_rx: oneshot::Receiver<()>,
}

impl SendThread {
    pub fn new(agent_id: Uuid, socket_tx: WriteHalf, sender_rx: SenderRX, stop_signal_rx: oneshot::Receiver<()>) -> Self {
        Self {
            agent_id,
            socket_tx,
            sender_rx,
            stop_signal_rx,
        }
    }

    pub async fn run(&mut self) {
        loop {
            select! {
                biased;
                packet = self.sender_rx.recv() => {
                    match packet {
                        Some(packet) => {
                            if self.socket_tx.send_packet(packet).await.is_err() {
                                logging_notice!(self.agent_id, "Send Thread", "Management side disconnected", "");
                                break;
                            }
                        },
                        None => {
                            logging_notice!(self.agent_id, "Send Thread", "Channel has been closed", "");
                            break;
                        },
                    }
                },
                _ = &mut self.stop_signal_rx => break,
            }
        }
        let _ = self.socket_tx.shutdown().await;
    }
}
