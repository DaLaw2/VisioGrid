use std::sync::Arc;
use std::net::TcpStream;
use std::thread::JoinHandle;
use std::sync::atomic::AtomicBool;
use crate::connection::packet::definition::Packet;
use crate::connection::connection_channel::definition;

struct ControlChannel {
    node_id: usize,
    socket: TcpStream,
    sender_handle: JoinHandle<()>,
    receiver_handle: JoinHandle<()>,
    pub stop_signal: Arc<AtomicBool>,
}

impl ControlChannel {
    pub fn new(node_id: usize, socket: TcpStream) -> Self {
        Self {
            node_id,
            socket,
            sender_handle: None,
            receiver_handle: None,
            stop_signal: Arc::new(AtomicBool::new(false))
        }
    }
}

impl definition::ConnectChannel for ControlChannel {
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