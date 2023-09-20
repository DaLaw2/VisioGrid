use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream, SocketAddr};
use crate::socket::definition;
use crate::socket::packet::base_packet::BasePacket;

struct Sender {
    id: usize,
    address: SocketAddr,
    stream: TcpStream
}

impl Sender {
    pub fn new(address: &SocketAddr, socket_id: usize) -> io::Result<Self> {
        Ok(Self {
            id: socket_id,
            address: address.clone(),
            stream: TcpStream::connect(address)?
        })
    }

    pub fn is_connect(&self) -> bool {
        self.stream.peer_addr().is_ok()
    }
}

impl definition::Sender for Sender {
    fn get_ip(&self) -> String {
        self.stream.peer_addr().map(|addr| addr.to_string()).unwrap_or_else()
    }

    fn get_socket_id(&self) -> usize {
        self.id
    }

    fn send_raw_data(&mut self, data: Vec<u8>) -> io::Result<()> {
        self.stream.write_all(&data)?;
        self.stream.flush()?;
        Ok(())
    }

    fn send_packet(&mut self, packet: BasePacket) -> io::Result<()> {
        self.stream.write_all(&packet.packet_length)?;
        self.stream.write_all(&packet.packet_id)?;
        self.stream.write_all(&packet.packet_data)?;
        Ok(())
    }
}

struct Receiver {
    id: usize,
    address: SocketAddr,
    listener: TcpListener,
    stream: TcpStream
}

impl Receiver {
    pub fn new() -> io::Result<Self> {

    }
}

impl definition::Receiver for Receiver {
    fn get_ip(&self) -> String {
        todo!()
    }

    fn get_socket_id(&self) -> usize {
        todo!()
    }

    fn receive_raw_data(&self) -> io::Result<Vec<u8>> {
        todo!()
    }

    fn receive_packet(&self) -> io::Result<BasePacket> {
        todo!()
    }
}