use std::fmt;
use std::fmt::Formatter;
use crate::manager::task::bounding_box::BoundingBox;
use crate::connection::packet::base_packet::BasePacket;
use crate::connection::packet::definition::{length_to_byte, Packet, PacketType};

pub struct BoundingBoxPacket {
    length: Vec<u8>,
    id: Vec<u8>,
    data: Vec<u8>,
    packet_type: PacketType,
}

impl BoundingBoxPacket {
    pub fn new(bounding_box: &BoundingBox) -> BoundingBoxPacket {
        let data = bounding_box.to_string();
        BoundingBoxPacket {
            length: length_to_byte(8 + 2 + data.len()),
            id: PacketType::BoundingBoxPacket.as_id_byte(),
            data: data.as_bytes().to_vec(),
            packet_type: PacketType::BoundingBoxPacket
        }
    }

    pub fn from_base_packet(base_packet: BasePacket) -> BoundingBoxPacket {
        BoundingBoxPacket {
            length: base_packet.length,
            id: base_packet.id,
            data: base_packet.data,
            packet_type: PacketType::BoundingBoxPacket
        }
    }
}

impl fmt::Display for BoundingBoxPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut length_array = [0_u8; 8];
        let mut id_array = [0_u8; 8];
        length_array.copy_from_slice(&self.length);
        id_array.copy_from_slice(&self.id);
        write!(f, "{} | {} | Data Length: {}", usize::from_be_bytes(length_array), usize::from_be_bytes(id_array), self.data.len())
    }
}

impl Packet for BoundingBoxPacket {
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