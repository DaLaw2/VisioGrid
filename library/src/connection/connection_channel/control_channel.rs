use tokio::sync::mpsc;
use tokio::net::TcpStream;
use crate::logger::logger::Logger;
use crate::logger::logger::LogLevel;
use crate::connection::packet::definition::Packet;
use crate::connection::connection_channel::definition;

struct ControlChannel {
    node_id: usize,
    socket: TcpStream,
    sender: mpsc::UnboundedSender<Option<Box<dyn Packet + Send>>>,
    receiver: mpsc::UnboundedReceiver<Option<Box<dyn Packet + Send>>>
}

impl ControlChannel {
    pub fn new(node_id: usize, socket: TcpStream) -> Self {
        let (send_tx, send_rx) = mpsc::unbounded_channel();
        let (recv_tx, recv_rx) = mpsc::unbounded_channel();
        Self {
            node_id,
            socket,
            sender: send_tx,
            receiver: recv_rx
        }

    }
}

impl definition::ConnectChannel for ControlChannel {
    fn disconnect(&mut self) {
        match self.sender.send(None) {
            Ok(_) => (),
            Err(_) => Logger::instance().append_node_log(self.node_id, LogLevel::ERROR, "Fail destroy Control Channel.".to_string())
        }
    }

    fn process_send<T: Packet>(&mut self, packet: T) {
        todo!()
    }

    fn process_receive<T: Packet>(&mut self, packet: T) {
       todo!()
    }
}