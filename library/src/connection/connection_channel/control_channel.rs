use tokio::sync::mpsc;
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::SendError;
use crate::logger::logger::{Logger, LogLevel};
use crate::connection::packet::definition::Packet;
use crate::connection::connection_channel::definition;
use crate::connection::connection_channel::receive_thread::ReceiveThread;
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::socket::node_socket::NodeSocket;
use crate::connection::connection_channel::send_thread::SendThread;

struct ControlChannel {
    node_id: usize,
    socket: NodeSocket,
    sender: mpsc::UnboundedSender<Option<Box<dyn Packet + Send>>>,
    receiver: Option<mpsc::UnboundedReceiver<BasePacket>>
}

impl ControlChannel {
    pub fn new(node_id: usize, socket: NodeSocket) -> Self {
        let (send_tx, send_rx) = mpsc::unbounded_channel();
        let (recv_tx, recv_rx) = mpsc::unbounded_channel();
        let mut send_thread = SendThread::new(node_id, socket.clone(), send_rx);
        let mut receive_thread = ReceiveThread::new(node_id, socket.clone(), recv_tx);
        tokio::spawn(async move {
            send_thread.run().await;
        });
        tokio::spawn(async move {
            receive_thread.run().await;
        });
        Self {
            node_id,
            socket,
            sender: send_tx,
            receiver: Some(recv_rx)
        }
    }
}

impl definition::ConnectChannel for ControlChannel {
    fn disconnect(&mut self) {
        match self.sender.send(None) {
            Ok(_) => {
                Logger::instance().append_node_log(self.node_id, LogLevel::INFO, "Control channel destroyed.".to_string());
                Logger::instance().append_system_log(LogLevel::INFO, format!("Node {}: Control channel destroyed.", self.node_id));
            },
            Err(_) => {
                Logger::instance().append_node_log(self.node_id, LogLevel::ERROR, "Fail destroy control channel.".to_string());
                Logger::instance().append_system_log(LogLevel::ERROR, format!("Node {}: Fail destroy control channel.", self.node_id));
            }
        }
        self.receiver = None;
    }

    fn process_send<T: Packet + Send>(&mut self, packet: &T) {
        match self.sender.send(Some(Box::new(packet))) {
            Ok(_) => {}
            Err(_) => {}
        }
    }

    fn process_receive<T: Packet + Send>(&mut self, packet: T) {
       todo!()
    }
}