use crate::socket::packet::base_packet::BasePacket;
use crate::socket::packet::definition::{Packet, PacketType};

pub struct InferenceTypePacket {
    packet_length: Vec<u8>,
    packet_id: Vec<u8>,
    packet_data: Vec<u8>,
    packet_type: PacketType,
}

impl InferenceTypePacket {
    // 記得改成傳入type enum
    pub fn new(inference_type: usize) -> InferenceTypePacket {
        InferenceTypePacket {
            packet_length: Self::length_to_byte(8 + 2 + inference_type.to_string().as_bytes().to_vec().len()),
            packet_id: PacketType::InferenceTypePacket.get_id(),
            packet_data: inference_type.to_string().as_bytes().to_vec(),
            packet_type: PacketType::InferenceTypePacket
        }
    }

    pub fn from_base_packet(base_packet: BasePacket) -> InferenceTypePacket {
        InferenceTypePacket {
            packet_length: base_packet.packet_length,
            packet_id: base_packet.packet_id,
            packet_data: base_packet.packet_data,
            packet_type: PacketType::InferenceTypePacket
        }
    }
}

impl Packet for InferenceTypePacket {
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