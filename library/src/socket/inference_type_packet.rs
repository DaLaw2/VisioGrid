use crate::socket::definition::{Packet, PacketType};

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
            packet_length: Self::length_to_byte(8 + 2 + inference_type.to_string().len()),
            packet_id: PacketType::InferenceTypePacket.get_id(),
            packet_data: inference_type.to_string().as_bytes().to_vec(),
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
        let length_string = self.packet_length.clone();
        let id_string = self.packet_id.clone();
        let length_string = String::from_utf8_lossy(&*length_string);
        let id_string = String::from_utf8_lossy(&*id_string);
        format!("{} | {} | Data Length: {}", length_string.to_string(), id_string.to_string(), self.packet_data.len())
    }

    fn equal(&self, packet_type: PacketType) -> bool {
        self.packet_type.eq(&packet_type)
    }
}