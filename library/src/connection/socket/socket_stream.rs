use std::io;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use crate::connection::packet::definition::Packet;
use crate::connection::packet::base_packet::BasePacket;

pub struct SocketStream {
    id: usize,
    address: SocketAddr,
    write_half: WriteHalf,
    read_half: ReadHalf
}

impl SocketStream {
    pub fn new(id: usize, socket: TcpStream) -> Self {
        let address = socket.peer_addr().expect(format!("Connection refuse. Socket ID: {}", id).as_str());
        let (read_half, write_half) = socket.into_split();
        Self {
            id,
            address,
            write_half: WriteHalf::new(write_half),
            read_half: ReadHalf::new(read_half)
        }
    }

    pub fn into_split(self) -> (WriteHalf, ReadHalf) {
        (self.write_half, self.read_half)
    }

    pub fn get_ip(&self) -> String {
        self.address.to_string()
    }

    pub fn get_socket_id(&self) -> usize {
        self.id
    }

    pub async fn send_raw_data(&mut self, data: &Vec<u8>) -> io::Result<()> {
        self.write_half.send_raw_data(data).await
    }

    pub async fn send_packet(&mut self, packet: Box<dyn Packet + Send>) -> io::Result<()> {
        self.write_half.send_packet(packet).await
    }

    pub async fn receive_raw_data(&mut self) -> io::Result<Vec<u8>> {
        self.read_half.receive_raw_data().await
    }

    pub async fn receive_packet(&mut self) -> io::Result<BasePacket> {
        self.read_half.receive_packet().await
    }
}

pub struct WriteHalf {
    write_half: OwnedWriteHalf
}

impl WriteHalf {
    pub fn new(write_half: OwnedWriteHalf) -> Self {
        WriteHalf {
            write_half
        }
    }

    pub async fn send_raw_data(&mut self, data: &Vec<u8>) -> io::Result<()> {
        self.write_half.write_all(&data).await?;
        self.write_half.flush().await?;
        Ok(())
    }

    pub async fn send_packet(&mut self, packet: Box<dyn Packet + Send>) -> io::Result<()> {
        let length = packet.as_length_byte();
        let id = packet.as_id_byte();
        let data = packet.as_data_byte();
        self.write_half.write_all(length).await?;
        self.write_half.write_all(id).await?;
        self.write_half.write_all(data).await?;
        self.write_half.flush().await?;
        Ok(())
    }
}

pub struct ReadHalf {
    read_half: OwnedReadHalf
}

impl ReadHalf {
    pub fn new(read_half: OwnedReadHalf) -> Self {
        ReadHalf {
            read_half
        }
    }

    pub async fn receive_raw_data(&mut self) -> io::Result<Vec<u8>> {
        let mut length_byte = [0_u8; 8];
        let mut id_byte = vec![0_u8; 8];
        self.read_half.read_exact(&mut length_byte).await?;
        self.read_half.read_exact(&mut id_byte).await?;
        let length = usize::from_be_bytes(length_byte);
        let mut data_byte = vec![0_u8; length - 16];
        self.read_half.read_exact(&mut data_byte).await?;
        let mut result = length_byte.to_vec();
        result.extend(id_byte);
        result.extend(data_byte);
        Ok(result)
    }

    pub async fn receive_packet(&mut self) -> io::Result<BasePacket> {
        let mut length_byte = [0_u8; 8];
        let mut id_byte = vec![0_u8; 8];
        self.read_half.read_exact(&mut length_byte).await?;
        self.read_half.read_exact(&mut id_byte).await?;
        let length = usize::from_be_bytes(length_byte);
        let mut data_byte = vec![0_u8; length - 16];
        self.read_half.read_exact(&mut data_byte).await?;
        let length_byte = length_byte.to_vec();
        Ok(BasePacket::new(length_byte, id_byte, data_byte))
    }
}
