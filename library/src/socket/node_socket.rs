use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream, SocketAddr};
use crate::socket::definition;
use crate::logger::logger::{Logger, LogLevel};
use crate::socket::packet::base_packet::BasePacket;

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
        let mut buffer = [0_u8; 1024];
        let mut result = Vec::new();
        if let Some(ref mut stream) = self.stream {
            loop {
                let size = stream.read(&mut buffer)?;
                if size == 0 {
                    break;
                }
                result.extend_from_slice(&buffer);
            }
            Ok(result)
        } else {
            let message = format!("Fail receive data from {:?}.", self.address);
            Logger::instance().append_system_log(LogLevel::ERROR, message);
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "No stream available."))
        }
    }

    fn receive_packet(&self) -> io::Result<BasePacket> {
        if let Some(ref mut stream) = self.stream {
            let mut length = vec![0_u8; 8];
            let mut id = vec![0_u8; 2];
            let mut data = Vec::new();
            stream.read_exact(&mut length)?;
            stream.read_exact(&mut id)?;
            stream.read_to_end(&mut data)?;
            Ok(BasePacket::new(length, id, data))
        } else {
            let message = format!("Fail receive data from {:?}.", self.address);
            Logger::instance().append_system_log(LogLevel::ERROR, message);
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "No stream available."))
        }
    }
}