use std::io;
use crate::connection::packet::base_packet::BasePacket;

pub trait Sender {
    fn get_ip(&self) -> String;
    fn get_socket_id(&self) -> usize;
    fn send_raw_data(&mut self, data: Vec<u8>) -> io::Result<()>;
    fn send_packet(&mut self, packet: BasePacket) -> io::Result<()>;
}

pub trait Receiver {
    fn get_ip(&self) -> String;
    fn get_socket_id(&self) -> usize;
    fn receive_raw_data(&mut self) -> io::Result<Vec<u8>>;
    fn receive_packet(&mut self) -> io::Result<BasePacket>;
}