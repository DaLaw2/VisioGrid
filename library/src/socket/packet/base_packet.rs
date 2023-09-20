use crate::socket::packet::definition::{Packet, PacketType};

pub struct BasePacket {
    pub packet_length: Vec<u8>,
    pub packet_id: Vec<u8>,
    pub packet_data: Vec<u8>,
    pub packet_type: PacketType,
}

impl BasePacket {
    pub fn new(packet_length: Vec<u8>, packet_id: Vec<u8>, packet_data: Vec<u8>) -> BasePacket {
        BasePacket {
            packet_length,
            packet_id: packet_id.clone(),
            packet_data,
            packet_type: PacketType::get_type(packet_id)
        }
    }
}

impl Packet for BasePacket {
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
        let mut length_array = [0_u8; 8];
        let mut id_array = [0_u8; 8];
        length_array.copy_from_slice(&self.packet_length);
        id_array.copy_from_slice(&self.packet_id);
        format!("{} | {} | Data Length: {}", usize::from_be_bytes(length_array), usize::from_be_bytes(id_array), self.packet_data.len())
    }

    fn equal(&self, packet_type: PacketType) -> bool {
        self.packet_type.eq(&packet_type)
    }
}