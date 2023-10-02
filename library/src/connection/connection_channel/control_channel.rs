use tokio::sync::mpsc;
use crate::logger::logger::{Logger, LogLevel};
use crate::connection::packet::definition::Packet;
use crate::connection::connection_channel::definition;
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::socket::node_socket::NodeSocket;
use crate::connection::connection_channel::send_thread::SendThread;
use crate::connection::connection_channel::receive_thread::ReceiveThread;

struct ControlChannel {
    node_id: usize,
    sender: mpsc::UnboundedSender<Option<Box<dyn Packet + Send>>>,
    receiver: Option<mpsc::UnboundedReceiver<BasePacket>>
}

impl ControlChannel {
    pub fn new(node_id: usize, socket: NodeSocket) -> Self {
        let (sender_tx, sender_rx) = mpsc::unbounded_channel();
        let (receiver_tx, receiver_rx) = mpsc::unbounded_channel();
        let (write_half, read_half) = socket.into_split();
        let mut send_thread = SendThread::new(node_id, write_half, sender_rx);
        let mut receive_thread = ReceiveThread::new(node_id, read_half, receiver_tx);
        tokio::spawn(async move {
            send_thread.run().await;
        });
        tokio::spawn(async move {
            receive_thread.run().await;
        });
        Self {
            node_id,
            sender: sender_tx,
            receiver: Some(receiver_rx)
        }
    }

    pub fn run(&mut self) {
    }
}

impl definition::ConnectChannel for ControlChannel {
    fn disconnect(&mut self) {
        match self.sender.send(None) {
            Ok(_) => {
                Logger::instance().append_node_log(self.node_id, LogLevel::INFO, "Control channel destroyed.".to_string());
            },
            Err(_) => {
                Logger::instance().append_node_log(self.node_id, LogLevel::ERROR, "Fail destroy control channel.".to_string());
                Logger::instance().append_system_log(LogLevel::ERROR, format!("Node {}: Fail destroy control channel.", self.node_id));
            }
        }
        self.receiver = None;
    }

    fn send<T: Packet + Send + 'static>(&mut self, packet: T) {
        let packet: Box<dyn Packet + Send + 'static> = Box::new(packet);
        match self.sender.send(Some(packet)) {
            Ok(_) => {
                Logger::instance().append_node_log(self.node_id, LogLevel::INFO, "Add packet to queue.".to_string());
            },
            Err(_) => {
                Logger::instance().append_node_log(self.node_id, LogLevel::ERROR, "Fail send packet to client.".to_string());
                Logger::instance().append_system_log(LogLevel::ERROR, format!("Node {}: Fail send packet to client.", self.node_id));
            }
        }
    }

    fn receive(&mut self, packet: BasePacket) {
    }
}