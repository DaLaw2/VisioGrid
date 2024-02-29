use tokio::sync::{mpsc, oneshot};
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::packet::Packet;
use crate::connection::socket::socket_stream::WriteHalf;
use crate::connection::channel::send_thread::SendThread;

pub type SenderTX = mpsc::UnboundedSender<Box<dyn Packet+Send>>;
pub type SenderRX = mpsc::UnboundedReceiver<Box<dyn Packet+Send>>;

pub struct DataChannelSender {
    sender_tx: SenderTX,
    stop_signal_tx: Option<oneshot::Sender<()>>,
}

impl DataChannelSender {
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
                let _ = stop_signal.send(());
                Logger::add_system_log(LogLevel::INFO, "Data Channel: Destroyed Sender successfully.".to_string()).await;
            },
            None => Logger::add_system_log(LogLevel::ERROR, "Data Channel: Failed to destroy Sender.".to_string()).await,
        }
    }

    pub async fn send<T: Packet + Send + 'static>(&mut self, packet: T) {
        let packet: Box<dyn Packet + Send + 'static> = Box::new(packet);
        if let Err(err) = self.sender_tx.send(packet) {
            Logger::add_system_log(LogLevel::ERROR, format!("Data Channel: Unable to submit packet to Send Thread.\nReason: {}", err)).await;
        }
    }
}
