use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream, SocketAddr};
use crate::connection::socket::definition;
use crate::logger::logger::{Logger, LogLevel};
use crate::connection::packet::base_packet::BasePacket;

struct Sender {
    id: usize,
    address: SocketAddr,
    stream: TcpStream
}

impl Sender {
    pub fn new(address: &SocketAddr, id: usize) -> io::Result<Self> {
        Ok(Self {
            id,
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
        self.address.to_string()
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
        self.stream.write_all(&packet.length)?;
        self.stream.write_all(&packet.id)?;
        self.stream.write_all(&packet.data)?;
        self.stream.flush()?;
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
    pub fn new(address: SocketAddr, id: usize) -> io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        let (stream, _) = listener.accept()?;
        Ok(Self {
            id,
            address,
            listener,
            stream
        })
    }
}

impl definition::Receiver for Receiver {
    fn get_ip(&self) -> String {
        self.address.to_string()
    }

    fn get_socket_id(&self) -> usize {
        self.id
    }

    fn receive_raw_data(&self) -> io::Result<Vec<u8>> {
        if let Some(ref mut stream) = self.stream {
            let mut length_byte = [0_u8; 8];
            let mut id_byte = vec![0_u8; 2];
            stream.read_exact(&mut length_byte)?;
            stream.read_exact(&mut id_byte)?;
            let length = usize::from_be_bytes(length_byte);
            let mut data_byte = vec![0_u8; length];
            stream.read_exact(&mut data_byte)?;
            let mut result = length_byte.to_vec();
            result.extend(id_byte).extend(data_byte);
            Ok(result)
        } else {
            let message = format!("Fail receive data from {:?}.", self.address);
            Logger::instance().append_system_log(LogLevel::ERROR, message);
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "No stream available."))
        }
    }

    fn receive_packet(&self) -> io::Result<BasePacket> {
        if let Some(ref mut stream) = self.stream {
            let mut length_byte = [0_u8; 8];
            let mut id_byte = vec![0_u8; 2];
            stream.read_exact(&mut length_byte)?;
            stream.read_exact(&mut id_byte)?;
            let length = usize::from_be_bytes(length_byte);
            let mut data_byte = vec![0_u8; length];
            stream.read_exact(&mut data_byte)?;
            let mut length_byte = length_byte.to_vec();
            Ok(BasePacket::new(length_byte, id_byte, data_byte))
        } else {
            let message = format!("Fail receive data from {:?}.", self.address);
            Logger::instance().append_system_log(LogLevel::ERROR, message);
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "No stream available."))
        }
    }
}