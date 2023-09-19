use crate::socket::definition::{BoundingBox, Packet, PacketType};

pub struct BoundingBoxPacket {
    packet_length: Vec<u8>,
    packet_id: Vec<u8>,
    packet_data: Vec<u8>,
    packet_type: PacketType,
}

impl BoundingBoxPacket {
    pub fn new(bounding_box: &BoundingBox) -> BoundingBoxPacket {
        let data = format!("Name: {}, X1: {}, X2: {}, Y1: {}, Y2: {}, Confidence: {}"
                           , bounding_box.name, bounding_box.x1
                           , bounding_box.x2, bounding_box.y1
                           , bounding_box.y2, bounding_box.confidence);
        BoundingBoxPacket {
            packet_length: Self::length_to_byte(8 + 2 + data.len()),
            packet_id: PacketType::BoundingBoxPacket.get_id(),
            packet_data: data.as_bytes().to_vec(),
            packet_type: PacketType::BoundingBoxPacket
        }
    }
}

impl Packet for BoundingBoxPacket {
    fn get_length_byte(&self) -> Vec<u8> {
        self.packet_length.clone()
    }

    fn get_id_byte(&self) -> Vec<u8> {
        self.packet_id.clone()
    }

    fn get_data_byte(&self) -> Vec<u8> {
        self.packet_data.clone()
    }

    fn get_data_string(&self) -> String {
        String::from_utf8_lossy(&*self.packet_data.clone()).to_string()
    }

    fn get_info(&self) -> String {
        let length_string = String::from_utf8_lossy(&*self.packet_length.clone());
        let id_string = String::from_utf8_lossy(&*self.packet_id.clone());
        format!("{} | {} | Data Length: {}", length_string.to_string(), id_string.to_string(), self.packet_data.len())
    }

    fn equal(&self, packet_type: &PacketType) -> bool {
        self.packet_type.eq(&packet_type)
    }
}