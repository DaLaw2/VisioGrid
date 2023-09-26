use tokio::sync::mpsc;
use crate::connection::packet::definition::Packet;
use crate::connection::socket::node_socket::NodeSocket;
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::packet::picture_packet::PicturePacket;
use crate::connection::connection_channel::definition::ConnectChannel;
use crate::connection::packet::inference_type_packet::InferenceTypePacket;
use crate::connection::packet::stop_inference_packet::StopInferencePacket;
use crate::connection::packet::data_channel_port_packet::DataChannelPortPacket;


pub struct SendThread<T: ConnectChannel> {
    socket: NodeSocket,
    receiver: mpsc::UnboundedReceiver<Option<Box<dyn Packet + Send>>>,
}

impl<T: ConnectChannel> SendThread<T> {
    pub fn new(socket: NodeSocket, receiver: mpsc::UnboundedReceiver<Option<Box<dyn Packet>>>) -> Self {
        Self {
            socket,
            receiver
        }
    }

    pub async fn run(&mut self) {
        while let Some(packet) = self.receiver.recv().await {
            match packet {
                Some(packet) => {
                    if let Ok(packet) = packet.as_any().downcast_ref::<BasePacket>() {
                        self.socket.send_packet(packet)
                    } else if let Ok(packet) = packet.as_any().downcast_ref::<DataChannelPortPacket>() {
                        self.socket.send_packet(packet)
                    } else if let Ok(packet) = packet.as_any().downcast_ref::<InferenceTypePacket>() {
                        self.socket.send_packet(packet)
                    } else if let Ok(packet) = packet.as_any().downcast_ref::<PicturePacket>() {
                        self.socket.send_packet(packet)
                    } else if let Ok(packet) = packet.as_any().downcast_ref::<StopInferencePacket>() {
                        self.socket.send_packet(packet)
                    }
                },
                None => break
            }
        }
    }
}