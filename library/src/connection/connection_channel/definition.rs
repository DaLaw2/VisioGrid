use crate::connection::packet::definition::Packet;
pub trait ConnectChannel {
    fn disconnect(&mut self);
    fn process_send<T: Packet>(&mut self, packet: T);
    fn process_receive<T: Packet>(&mut self, packet: T);
}