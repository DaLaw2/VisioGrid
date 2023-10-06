use std::fmt;
use std::fmt::Formatter;
use crate::connection::packet::definition::{Packet, PacketType};

pub struct BasePacket {
    pub(crate) length: Vec<u8>,
    pub(crate) id: Vec<u8>,
    pub(crate) data: Vec<u8>,
    pub(crate) packet_type: PacketType,
}

impl BasePacket {
    pub fn new(length: Vec<u8>, id: Vec<u8>, data: Vec<u8>) -> BasePacket {
        BasePacket {
            length,
            id: id.clone(),
            data,
            packet_type: PacketType::get_packet_type(&id)
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
    fn as_length_byte(&self) -> &[u8] {
        &self.length
    }

    fn as_id_byte(&self) -> &[u8] {
        &self.id
    }

    fn as_data_byte(&self) -> &[u8] {
        &self.data
    }

    fn clone_length_byte(&self) -> Vec<u8> {
        self.length.clone()
    }

    fn clone_id_byte(&self) -> Vec<u8> {
        self.id.clone()
    }

    fn clone_data_byte(&self) -> Vec<u8> {
        self.data.clone()
    }

    fn data_to_string(&self) -> String {
        String::from_utf8_lossy(&*self.data.clone()).to_string()
    }

    fn get_packet_type(&self) -> PacketType {
        self.packet_type
    }

    fn equal(&self, packet_type: PacketType) -> bool {
        self.packet_type.eq(&packet_type)
    }
}
