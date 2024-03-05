use uuid::Uuid;
use tokio::sync::{mpsc, oneshot};
use crate::connection::packet::Packet;
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::socket::socket_stream::WriteHalf;
use crate::connection::channel::send_thread::SendThread;

pub type SenderTX = mpsc::UnboundedSender<Box<dyn Packet+Send>>;
pub type SenderRX = mpsc::UnboundedReceiver<Box<dyn Packet+Send>>;

pub struct ControlChannelSender {
    agent_id: Uuid,
    sender_tx: SenderTX,
    stop_signal_tx: Option<oneshot::Sender<()>>,
}

impl ControlChannelSender {
    pub fn new(agent_id: Uuid, socket_tx: WriteHalf) -> Self {
        let (sender_tx, sender_rx) = mpsc::unbounded_channel();
        let (stop_signal_tx, stop_signal_rx) = oneshot::channel();
        let mut send_thread = SendThread::new(agent_id, socket_tx, sender_rx, stop_signal_rx);
        tokio::spawn(async move {
            send_thread.run().await;
        });
        Self {
            agent_id,
            sender_tx,
            stop_signal_tx: Some(stop_signal_tx),
        }
    }

    pub async fn disconnect(&mut self) {
        match self.stop_signal_tx.take() {
            Some(stop_signal) => {
                match stop_signal.send(()) {
                    Ok(_) => Logger::add_agent_log(self.agent_id, LogLevel::INFO, "Control Channel: Destroyed Sender successfully.".to_string()).await,
                    Err(_) => Logger::add_agent_log(self.agent_id, LogLevel::ERROR, "Control Channel: Failed to destroy Sender.".to_string()).await,
                }
            },
            None => Logger::add_agent_log(self.agent_id, LogLevel::ERROR, "Control Channel: Failed to destroy Sender.".to_string()).await,
        }
    }

    pub async fn send<T: Packet + Send + 'static>(&mut self, packet: T) {
        let packet: Box<dyn Packet + Send + 'static> = Box::new(packet);
        if let Err(err) = self.sender_tx.send(packet) {
            Logger::add_agent_log(self.agent_id, LogLevel::ERROR, format!("Control Channel: Unable to submit packet to Send Thread.\nReason: {}", err)).await;
        }
    }
}