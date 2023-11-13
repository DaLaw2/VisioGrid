use tokio::sync::{mpsc, oneshot};
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::packet::definition::Packet;
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::connection_channel::send_thread::SendThread;
use crate::connection::connection_channel::data_channel_receive_thread::ReceiveThread;
use crate::connection::connection_channel::data_packet_channel::{DataPacketChannel, PacketReceiver};

pub struct DataChannel {
    node_id: usize,
    sender: mpsc::UnboundedSender<Option<Box<dyn Packet + Send>>>,
    receiver: Option<mpsc::UnboundedReceiver<BasePacket>>,
    stop_signal: Option<oneshot::Sender<()>>,
}

impl DataChannel {
    pub fn new(node_id: usize, socket: SocketStream) -> (Self, PacketReceiver) {
        let (sender_tx, sender_rx) = mpsc::unbounded_channel();
        let (stop_signal_tx, stop_signal_rx) = oneshot::channel();
        let (socket_sender, socket_receiver) = socket.into_split();
        let (data_packet_channel_tx, data_packet_channel_rx) = DataPacketChannel::split();
        let mut send_thread = SendThread::new(node_id, socket_sender, sender_rx);
        let mut receive_thread = ReceiveThread::new(node_id, socket_receiver, stop_signal_rx, data_packet_channel_tx);
        tokio::spawn(async move {
            send_thread.run().await;
        });
        tokio::spawn(async move {
            receive_thread.run().await;
        });
        let data_channel = Self {
            node_id,
            sender: sender_tx,
            receiver: Some(receiver_rx),
            stop_signal: Some(stop_signal_tx)
        };
        (data_channel, data_packet_channel_rx)
    }

    pub async fn run(&mut self) {
        let mut receiver = match self.receiver.take() {
            Some(receiver) => receiver,
            None => {
                Logger::append_node_log(self.node_id, LogLevel::ERROR, "Data Channel: Fail to run Data Channel because it's closed.".to_string()).await;
                return;
            }
        };
        tokio::spawn(async move {
            while let Some(_packet) = receiver.recv().await {
                //Process receive not yet complete
                unimplemented!()
            }
        });
    }

    pub async fn disconnect(&mut self) {
        match self.sender.send(None) {
            Ok(_) => Logger::append_node_log(self.node_id, LogLevel::INFO, "Data Channel: Destroy Sender successfully.".to_string()).await,
            Err(_) => Logger::append_node_log(self.node_id, LogLevel::ERROR, "Data Channel: Fail to destroy Sender.".to_string()).await
        }
        match self.stop_signal.take() {
            Some(stop_signal) => {
                let _ = stop_signal.send(());
                Logger::append_node_log(self.node_id, LogLevel::INFO, "Data Channel: Destroy Receiver successfully.".to_string()).await;
            },
            None => Logger::append_node_log(self.node_id, LogLevel::ERROR, "Data Channel: Fail to destroy Receiver.".to_string()).await
        }
    }

    pub async fn send<T: Packet + Send + 'static>(&mut self, packet: T) {
        let packet: Box<dyn Packet + Send + 'static> = Box::new(packet);
        match self.sender.send(Some(packet)) {
            Ok(_) => {},
            Err(_) => Logger::append_node_log(self.node_id, LogLevel::ERROR, "Data Channel: Failed to send packet to client.".to_string()).await
        }
    }
}
