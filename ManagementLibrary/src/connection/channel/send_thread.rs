use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use crate::connection::packet::Packet;
use crate::connection::socket::socket_stream::WriteHalf;

type SenderRX = mpsc::UnboundedReceiver<Box<dyn Packet + Send>>;

pub struct SendThread {
    socket_tx: WriteHalf,
    sender_rx: SenderRX,
    stop_signal_rx: oneshot::Receiver<()>,
}

impl SendThread {
    pub fn new(socket_tx: WriteHalf, sender_rx: SenderRX, stop_signal_rx: oneshot::Receiver<()>) -> Self {
        Self {
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
                                break;
                            }
                        },
                        None => break,
                    }
                },
                _ = &mut self.stop_signal_rx => break,
            }
        }
        let _ = self.socket_tx.shutdown().await;
    }
}
