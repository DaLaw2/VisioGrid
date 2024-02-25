use std::io;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use crate::connection::packet::Packet;
use crate::connection::packet::base_packet::BasePacket;

pub struct SocketStream {
    read_half: ReadHalf,
    write_half: WriteHalf,
}

impl SocketStream {
    pub fn new(socket: TcpStream) -> Self {
        let (read_half, write_half) = socket.into_split();
        Self {
            read_half: ReadHalf::new(read_half),
            write_half: WriteHalf::new(write_half),
        }
    }

    pub fn into_split(self) -> (WriteHalf, ReadHalf) {
        (self.write_half, self.read_half)
    }
}

pub struct WriteHalf {
    write_half: OwnedWriteHalf,
}

impl WriteHalf {
    pub fn new(write_half: OwnedWriteHalf) -> Self {
        Self {
            write_half,
        }
    }

    pub async fn shutdown(&mut self) -> io::Result<()> {
        self.write_half.shutdown().await
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
    read_half: OwnedReadHalf,
}

impl ReadHalf {
    pub fn new(read_half: OwnedReadHalf) -> Self {
        Self {
            read_half,
        }
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
