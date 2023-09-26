use tokio::sync::mpsc;
use crate::connection::packet::definition::Packet;
use crate::connection::socket::node_socket::NodeSocket;
use crate::connection::connection_channel::definition::ConnectChannel;
use crate::logger::logger::{Logger, LogLevel};

pub struct ReceiveThread<T: ConnectChannel> {
    socket: NodeSocket,
    sender: mpsc::UnboundedSender<Option<Box<dyn Packet + Send>>>,
}

impl<T: ConnectChannel> ReceiveThread<T> {
    pub fn new(socket: NodeSocket, sender: mpsc::UnboundedSender<Option<Box<dyn Packet>>>) -> Self {
        Self {
            socket,
            sender
        }
    }

    pub async fn run(&mut self) {
        loop {
            match self.socket.receive_packet().await {
                Ok(packet) => {

                },
                Err(_) => {
                    Logger::instance().append_system_log(LogLevel::ERROR, "")
                }
            }
        }
    }
}