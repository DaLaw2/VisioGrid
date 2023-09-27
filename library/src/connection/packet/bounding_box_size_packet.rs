use std::fmt;
use std::any::Any;
use std::fmt::Formatter;
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::packet::definition::{Packet, PacketType, length_to_byte};

pub struct BoundingBoxSizePacket {
    length: Vec<u8>,
    id: Vec<u8>,
    data: Vec<u8>,
    packet_type: PacketType,
}

impl BoundingBoxSizePacket {
    pub fn new(amount: usize) -> BoundingBoxSizePacket {
        BoundingBoxSizePacket {
            length: length_to_byte(8 + 2 + amount.to_string().len()),
            id: PacketType::BoundingBoxSizePacket.get_id(),
            data: amount.to_string().as_bytes().to_vec(),
            packet_type: PacketType::BoundingBoxSizePacket
        }
    }

    pub fn from_base_packet(base_packet: BasePacket) -> BoundingBoxSizePacket {
        BoundingBoxSizePacket {
            length: base_packet.length,
            id: base_packet.id,
            data: base_packet.data,
            packet_type: PacketType::BoundingBoxSizePacket
        }
    }
}

impl fmt::Display for BoundingBoxSizePacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut length_array = [0_u8; 8];
        let mut id_array = [0_u8; 8];
        length_array.copy_from_slice(&self.length);
        id_array.copy_from_slice(&self.id);
        write!(f, "{} | {} | Data Length: {}", usize::from_be_bytes(length_array), usize::from_be_bytes(id_array), self.data.len())
    }
}

impl Packet for BoundingBoxSizePacket {
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