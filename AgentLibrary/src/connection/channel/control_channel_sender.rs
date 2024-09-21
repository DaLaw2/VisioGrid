use crate::connection::channel::send_thread::SendThread;
use crate::connection::packet::Packet;
use crate::connection::socket::socket_stream::WriteHalf;
use crate::utils::logging::*;
use tokio::sync::{mpsc, oneshot};

pub type SenderTX = mpsc::UnboundedSender<Box<dyn Packet + Send>>;
pub type SenderRX = mpsc::UnboundedReceiver<Box<dyn Packet + Send>>;

pub struct ControlChannelSender {
    sender_tx: SenderTX,
    stop_signal_tx: Option<oneshot::Sender<()>>,
}

impl ControlChannelSender {
    pub fn new(socket_tx: WriteHalf) -> Self {
        let (sender_tx, sender_rx) = mpsc::unbounded_channel();
        let (stop_signal_tx, stop_signal_rx) = oneshot::channel();
        let mut send_thread = SendThread::new(socket_tx, sender_rx, stop_signal_rx);
        tokio::spawn(async move {
            send_thread.run().await;
        });
        Self {
            sender_tx,
            stop_signal_tx: Some(stop_signal_tx),
        }
    }

    pub async fn disconnect(&mut self) {
        match self.stop_signal_tx.take() {
            Some(stop_signal) => {
                if stop_signal.send(()).is_err() {
                    logging_error!(NetworkEntry::DestroyInstanceError);
                }
            }
            None => logging_error!(NetworkEntry::DestroyInstanceError),
        }
    }

    pub async fn send<T: Packet + Send + 'static>(&mut self, packet: T) {
        let packet: Box<dyn Packet + Send + 'static> = Box::new(packet);
        if self.sender_tx.send(packet).is_err() {
            logging_information!(NetworkEntry::ChannelClosed);
        }
    }
}
