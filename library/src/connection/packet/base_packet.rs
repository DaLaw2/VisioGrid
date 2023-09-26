use std::fmt;
use std::any::Any;
use std::fmt::Formatter;
use crate::connection::packet::definition::{Packet, PacketType};

pub struct BasePacket {
    pub length: Vec<u8>,
    pub id: Vec<u8>,
    pub data: Vec<u8>,
    pub packet_type: PacketType,
}

impl BasePacket {
    pub fn new(length: Vec<u8>, id: Vec<u8>, data: Vec<u8>) -> BasePacket {
        BasePacket {
            length,
            id: id.clone(),
            data,
            packet_type: PacketType::get_type(id)
        }
    }
}

impl fmt::Display for BasePacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut length_array = [0_u8; 8];
        let mut id_array = [0_u8; 8];
        length_array.copy_from_slice(&self.length);
        id_array.copy_from_slice(&self.id);
        write!(f, "{} | {} | Data Length: {}", usize::from_be_bytes(length_array), usize::from_be_bytes(id_array), self.data.len())
    }
}

impl Packet for BasePacket {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_length_byte(&self) -> Vec<u8> {
        self.length.clone()
    }

    fn get_id_byte(&self) -> Vec<u8> {
        self.id.clone()
    }

    fn get_data_byte(&self) -> Vec<u8> {
        self.data.clone()
    }

    fn get_data_string(&self) -> String {
        String::from_utf8_lossy(&*self.data.clone()).to_string()
    }

    fn equal(&self, packet_type: PacketType) -> bool {
        self.packet_type.eq(&packet_type)
    }
}