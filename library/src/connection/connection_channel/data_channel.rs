use std::sync::Arc;
use std::net::TcpStream;
use std::sync::atomic::AtomicBool;
use crate::connection::packet::definition::Packet;
use crate::connection::connection_channel::definition;

struct DataChannel {
    node_id: usize,
    socket: TcpStream,
    stop_signal: Arc<AtomicBool>,
}

impl DataChannel {
    pub fn new(node_id: usize, socket: TcpStream) -> Self {
        Self {
            node_id,
            socket,
            stop_signal: Arc::new(AtomicBool::new(false))
        }
    }
}

impl definition::ConnectChannel for DataChannel {
    fn disconnect(&mut self) {
        todo!()
    }

    fn process_send<T: Packet>(&mut self, packet: T) {
        todo!()
    }

    fn process_receive<T: Packet>(&mut self, packet: T) {
        todo!()
    }
}