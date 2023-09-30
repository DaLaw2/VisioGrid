use crate::connection::packet::definition::Packet;
use crate::connection::packet::base_packet::BasePacket;

pub trait ConnectChannel {
    fn disconnect(&mut self);
    fn process_send(&mut self, packet: &(dyn Packet + Send));
    fn process_receive(&mut self, packet: BasePacket);
}