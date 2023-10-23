use tokio::sync::{mpsc, oneshot};
use crate::utils::logger::{Logger, LogLevel};
use crate::connection::packet::definition::Packet;
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::socket::socket_stream::SocketStream;
use crate::connection::connection_channel::send_thread::SendThread;
use crate::connection::connection_channel::receive_thread::ReceiveThread;

pub struct DataChannel {
    node_id: usize,
    sender: mpsc::UnboundedSender<Option<Box<dyn Packet + Send>>>,
    receiver: Option<mpsc::UnboundedReceiver<BasePacket>>,
    stop_signal: Option<oneshot::Sender<()>>
}

impl DataChannel {
    pub fn new(node_id: usize, socket: SocketStream) -> Self {
        let (sender_tx, sender_rx) = mpsc::unbounded_channel();
        let (receiver_tx, receiver_rx) = mpsc::unbounded_channel();
        let (stop_signal_tx, stop_signal_rx) = oneshot::channel();
        let (socket_receiver, socket_sender) = socket.into_split();
        let mut send_thread = SendThread::new(node_id, socket_sender, sender_rx);
        let mut receive_thread = ReceiveThread::new(node_id, socket_receiver, receiver_tx, stop_signal_rx);
        tokio::spawn(async move {
            send_thread.run().await;
        });
        tokio::spawn(async move {
            receive_thread.run().await;
        });
        Self {
            node_id,
            sender: sender_tx,
            receiver: Some(receiver_rx),
            stop_signal: Some(stop_signal_tx)
        }
    }

    pub async fn run(&mut self) {
        let mut receiver = self.receiver.take().expect("Data channel has been closed.");
        tokio::spawn(async move {
            while let Some(_packet) = receiver.recv().await {
                //Process receive not yet complete
                unimplemented!()
            }
        });
    }

    pub async fn disconnect(&mut self) {
        match self.sender.send(None) {
            Ok(_) => {
                Logger::instance().await.append_node_log(self.node_id, LogLevel::INFO, "Data channel destroyed.".to_string());
            },
            Err(_) => {
                Logger::instance().await.append_node_log(self.node_id, LogLevel::ERROR, "Fail destroy control channel.".to_string());
                Logger::instance().await.append_system_log(LogLevel::ERROR, format!("Node {}: Fail destroy control channel.", self.node_id));
            }
        }
        let _ = self.stop_signal.take().expect("Data channel has been closed.").send(());
    }

    pub async fn send<T: Packet + Send + 'static>(&mut self, packet: T) {
        let packet: Box<dyn Packet + Send + 'static> = Box::new(packet);
        match self.sender.send(Some(packet)) {
            Ok(_) => {
                Logger::instance().await.append_node_log(self.node_id, LogLevel::INFO, "Add packet to queue.".to_string());
            },
            Err(_) => {
                Logger::instance().await.append_node_log(self.node_id, LogLevel::ERROR, "Fail send packet to client.".to_string());
                Logger::instance().await.append_system_log(LogLevel::ERROR, format!("Node {}: Fail send packet to client.", self.node_id));
            }
        }
    }
}
