use crate::connection::packet::{Packet, PacketType};

pub struct BasePacket {
    pub length: Vec<u8>,
    pub id: Vec<u8>,
    pub data: Vec<u8>,
    pub packet_type: PacketType,
}

impl BasePacket {
    pub fn new(length: Vec<u8>, id: Vec<u8>, data: Vec<u8>) -> Self {
        let packet_type = PacketType::parse_packet_type(&id);
        Self {
            length,
            id,
            data,
            packet_type,
        }
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

    fn packet_type(&self) -> PacketType {
        self.packet_type
    }

    fn equal(&self, packet_type: PacketType) -> bool {
        self.packet_type.eq(&packet_type)
    }
}
