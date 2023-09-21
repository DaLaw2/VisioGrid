use crate::connection::packet::base_packet::BasePacket;
use crate::connection::packet::definition::{Packet, PacketType};

pub struct BoundingBox {
    pub x1: f64,
    pub x2: f64,
    pub y1: f64,
    pub y2: f64,
    pub confidence: f64,
    pub name: String
}

pub struct BoundingBoxPacket {
    length: Vec<u8>,
    id: Vec<u8>,
    data: Vec<u8>,
    packet_type: PacketType,
}

impl BoundingBoxPacket {
    pub fn new(bounding_box: &BoundingBox) -> BoundingBoxPacket {
        let data = format!("Name: {}, X1: {}, X2: {}, Y1: {}, Y2: {}, Confidence: {}"
                           , bounding_box.name, bounding_box.x1
                           , bounding_box.x2, bounding_box.y1
                           , bounding_box.y2, bounding_box.confidence);
        BoundingBoxPacket {
            length: Self::length_to_byte(8 + 2 + data.len()),
            id: PacketType::BoundingBoxPacket.get_id(),
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

impl Packet for BoundingBoxPacket {
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

    fn get_info(&self) -> String {
        let mut length_array = [0_u8; 8];
        let mut id_array = [0_u8; 8];
        length_array.copy_from_slice(&self.length);
        id_array.copy_from_slice(&self.id);
        format!("{} | {} | Data Length: {}", usize::from_be_bytes(length_array), usize::from_be_bytes(id_array), self.data.len())
    }

    fn equal(&self, packet_type: PacketType) -> bool {
        self.packet_type.eq(&packet_type)
    }
}