use crate::connection::packet::definition::Packet;
use crate::connection::packet::base_packet::BasePacket;

pub trait ConnectChannel {
    fn disconnect(&mut self);
    fn process_send<T: Packet>(&mut self, packet: T);
    fn process_receive(&mut self, packet: BasePacket);
}