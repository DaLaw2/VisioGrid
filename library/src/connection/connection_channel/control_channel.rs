use uuid::Uuid;
use tokio::sync::{mpsc, oneshot};
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::packet::definition::Packet;
use crate::connection::connection_channel::sender::Sender;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::connection_channel::control_channel_receiver::Receiver;
use crate::connection::connection_channel::control_packet_channel::{ControlPacketChannel, PacketReceiver};

pub struct ControlChannel {
    node_id: Uuid,
    sender: mpsc::UnboundedSender<Option<Box<dyn Packet + Send>>>,
    stop_signal: Option<oneshot::Sender<()>>,
}

impl ControlChannel {
    pub fn new(node_id: Uuid, socket: SocketStream) -> (Self, PacketReceiver) {
        let (socket_sender, socket_receiver) = socket.into_split();
        let (sender_tx, sender_rx) = mpsc::unbounded_channel();
        let (stop_signal_tx, stop_signal_rx) = oneshot::channel();
        let (control_packet_channel_tx, control_packet_channel_rx) = ControlPacketChannel::split();
        let mut sender = Sender::new(node_id, socket_sender, sender_rx);
        let mut receiver = Receiver::new(node_id, socket_receiver, stop_signal_rx, control_packet_channel_tx);
        tokio::spawn(async move {
            sender.run().await;
        });
        tokio::spawn(async move {
            receiver.run().await;
        });
        let control_channel = Self {
            node_id,
            sender: sender_tx,
            stop_signal: Some(stop_signal_tx),
        };
        (control_channel, control_packet_channel_rx)
    }

    pub async fn disconnect(&mut self) {
        match self.sender.send(None) {
            Ok(_) => Logger::append_node_log(self.node_id, LogLevel::INFO, "Control Channel: Destroyed Sender successfully.".to_string()).await,
            Err(err) => Logger::append_node_log(self.node_id, LogLevel::ERROR, format!("Control Channel: Failed to destroy Sender.\nReason: {}.", err)).await,
        }
        match self.stop_signal.take() {
            Some(stop_signal) => {
                let _ = stop_signal.send(());
                Logger::append_node_log(self.node_id, LogLevel::INFO, "Control Channel: Destroyed Receiver successfully.".to_string()).await;
            },
            None => Logger::append_node_log(self.node_id, LogLevel::ERROR, "Control Channel: Failed to destroy Receiver.".to_string()).await,
        }
    }

    pub async fn send<T: Packet + Send + 'static>(&mut self, packet: T) {
        let packet: Box<dyn Packet + Send + 'static> = Box::new(packet);
        if let Err(err) = self.sender.send(Some(packet)) {
            Logger::append_node_log(self.node_id, LogLevel::ERROR, format!("Control Channel: Failed to send packet to client.\nReason: {}.", err)).await;
        }
    }
}
