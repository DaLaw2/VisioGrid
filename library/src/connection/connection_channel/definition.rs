use crate::connection::packet::definition::Packet;
use crate::connection::packet::base_packet::BasePacket;

pub trait ConnectChannel {
    fn disconnect(&mut self);
    fn send<T: Packet + Send + 'static>(&mut self, packet: T);
    fn receive(&mut self, packet: BasePacket);
}