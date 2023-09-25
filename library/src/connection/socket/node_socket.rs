use std::io::{self, Read, Write};
use std::net::{TcpStream, SocketAddr};
use crate::connection::packet::base_packet::BasePacket;

pub struct NodeSocket {
    id: usize,
    address: SocketAddr,
    socket: TcpStream
}

impl NodeSocket {
    pub fn new(id: usize, socket: TcpStream) -> Self {
        Self {
            id,
            address: socket.peer_addr().expect(format!("Connection refuse. Socket ID: {}", id).as_str()),
            socket
        }
    }

    pub fn is_connect(&self) -> bool {
        self.socket.peer_addr().is_ok()
    }

    pub fn get_ip(&self) -> String {
        self.address.to_string()
    }

    pub fn get_socket_id(&self) -> usize {
        self.id
    }

    pub fn send_raw_data(&mut self, data: Vec<u8>) -> io::Result<()> {
        self.socket.write_all(&data)?;
        self.socket.flush()?;
        Ok(())
    }

    pub fn send_packet(&mut self, packet: BasePacket) -> io::Result<()> {
        self.socket.write_all(&packet.length)?;
        self.socket.write_all(&packet.id)?;
        self.socket.write_all(&packet.data)?;
        self.socket.flush()?;
        Ok(())
    }

    pub fn receive_raw_data(&mut self) -> io::Result<Vec<u8>> {
        let mut length_byte = [0_u8; 8];
        let mut id_byte = vec![0_u8; 2];
        self.socket.read_exact(&mut length_byte)?;
        self.socket.read_exact(&mut id_byte)?;
        let length = usize::from_be_bytes(length_byte);
        let mut data_byte = vec![0_u8; length - 10];
        self.socket.read_exact(&mut data_byte)?;
        let mut result = length_byte.to_vec();
        result.extend(id_byte);
        result.extend(data_byte);
        Ok(result)
    }

    pub fn receive_packet(&mut self) -> io::Result<BasePacket> {
        let mut length_byte = [0_u8; 8];
        let mut id_byte = vec![0_u8; 2];
        self.socket.read_exact(&mut length_byte)?;
        self.socket.read_exact(&mut id_byte)?;
        let length = usize::from_be_bytes(length_byte);
        let mut data_byte = vec![0_u8; length - 10];
        self.socket.read_exact(&mut data_byte)?;
        let length_byte = length_byte.to_vec();
        Ok(BasePacket::new(length_byte, id_byte, data_byte))
    }
}