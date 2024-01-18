use uuid::Uuid;
use tokio::sync::{mpsc, oneshot};
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::packet::definition::Packet;
use crate::connection::connection_channel::sender::Sender;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::connection_channel::data_channel_receiver::Receiver;
use crate::connection::connection_channel::data_packet_channel::{DataPacketChannel, PacketReceiver};

pub struct DataChannel {
    node_id: Uuid,
    sender: mpsc::UnboundedSender<Option<Box<dyn Packet + Send>>>,
    stop_signal: Option<oneshot::Sender<()>>,
}

impl DataChannel {
    pub fn new(node_id: Uuid, socket: SocketStream) -> (Self, PacketReceiver) {
        let (sender_tx, sender_rx) = mpsc::unbounded_channel();
        let (stop_signal_tx, stop_signal_rx) = oneshot::channel();
        let (socket_sender, socket_receiver) = socket.into_split();
        let (data_packet_channel_tx, data_packet_channel_rx) = DataPacketChannel::split();
        let mut sender = Sender::new(node_id, socket_sender, sender_rx);
        let mut receiver = Receiver::new(node_id, socket_receiver, stop_signal_rx, data_packet_channel_tx);
        tokio::spawn(async move {
            sender.run().await;
        });
        tokio::spawn(async move {
            receiver.run().await;
        });
        let data_channel = Self {
            node_id,
            sender: sender_tx,
            stop_signal: Some(stop_signal_tx)
        };
        (data_channel, data_packet_channel_rx)
    }

    pub async fn disconnect(&mut self) {
        match self.sender.send(None) {
            Ok(_) => Logger::append_node_log(self.node_id, LogLevel::INFO, "Data Channel: Destroyed Sender successfully.".to_string()).await,
            Err(_) => Logger::append_node_log(self.node_id, LogLevel::ERROR, "Data Channel: Failed to destroy Sender.".to_string()).await
        }
        match self.stop_signal.take() {
            Some(stop_signal) => {
                let _ = stop_signal.send(());
                Logger::append_node_log(self.node_id, LogLevel::INFO, "Data Channel: Destroyed Receiver successfully.".to_string()).await;
            },
            None => Logger::append_node_log(self.node_id, LogLevel::ERROR, "Data Channel: Failed to destroy Receiver.".to_string()).await
        }
    }

    pub async fn send<T: Packet + Send + 'static>(&mut self, packet: T) {
        let packet: Box<dyn Packet + Send + 'static> = Box::new(packet);
        if let Err(err) = self.sender.send(Some(packet)) {
            Logger::append_node_log(self.node_id, LogLevel::ERROR, format!("Data Channel: Failed to send packet to client. Reason: {}", err)).await;
        }
    }
}
