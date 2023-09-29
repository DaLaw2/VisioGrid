use std::io;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::packet::definition::Packet;

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

    pub async fn send_raw_data(&mut self, data: &Vec<u8>) -> io::Result<()> {
        self.socket.write_all(&data).await?;
        self.socket.flush().await?;
        Ok(())
    }

    pub async fn send_packet(&mut self, packet: &dyn Packet) -> io::Result<()> {
        self.socket.write_all(&packet.as_length_byte()).await?;
        self.socket.write_all(&packet.as_id_byte()).await?;
        self.socket.write_all(&packet.as_data_byte()).await?;
        self.socket.flush().await?;
        Ok(())
    }

    pub async fn receive_raw_data(&mut self) -> io::Result<Vec<u8>> {
        let mut length_byte = [0_u8; 8];
        let mut id_byte = vec![0_u8; 2];
        self.socket.read_exact(&mut length_byte).await?;
        self.socket.read_exact(&mut id_byte).await?;
        let length = usize::from_be_bytes(length_byte);
        let mut data_byte = vec![0_u8; length - 10];
        self.socket.read_exact(&mut data_byte).await?;
        let mut result = length_byte.to_vec();
        result.extend(id_byte);
        result.extend(data_byte);
        Ok(result)
    }

    pub async fn receive_packet(&mut self) -> io::Result<BasePacket> {
        let mut length_byte = [0_u8; 8];
        let mut id_byte = vec![0_u8; 2];
        self.socket.read_exact(&mut length_byte).await?;
        self.socket.read_exact(&mut id_byte).await?;
        let length = usize::from_be_bytes(length_byte);
        let mut data_byte = vec![0_u8; length - 10];
        self.socket.read_exact(&mut data_byte).await?;
        let length_byte = length_byte.to_vec();
        Ok(BasePacket::new(length_byte, id_byte, data_byte))
    }
}