use std::fmt;

pub trait Packet: fmt::Display + Send {
    fn as_length_byte(&self) -> &[u8];
    fn as_id_byte(&self) -> &[u8];
    fn as_data_byte(&self) -> &[u8];
    fn clone_length_byte(&self) -> Vec<u8>;
    fn clone_id_byte(&self) -> Vec<u8>;
    fn clone_data_byte(&self) -> Vec<u8>;
    fn data_to_string(&self) -> String;
    fn get_packet_type(&self) -> PacketType;
    fn equal(&self, packet_type: PacketType) -> bool;
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum PacketType {
    BasePacket,
    BoundingBoxPacket,
    DataChannelPortPacket,
    FileBodyPacket,
    FileHeaderPacket,
    InferenceTypePacket,
    StopInferencePacket,
    StopInferenceReturnPacket
}

impl PacketType {
    pub fn as_id_byte(&self) -> Vec<u8> {
        let id: usize = match self {
            PacketType::BasePacket => 0,
            PacketType::BoundingBoxPacket => 1,
            PacketType::DataChannelPortPacket => 3,
            PacketType::FileBodyPacket => 4,
            PacketType::FileHeaderPacket => 5,
            PacketType::InferenceTypePacket => 6,
            PacketType::StopInferencePacket => 7,
            PacketType::StopInferenceReturnPacket => 8,
        };
        id.to_be_bytes().to_vec()
    }

    pub fn get_packet_type(byte: &Vec<u8>) -> PacketType {
        let mut byte_array = [0_u8; 8];
        byte_array.copy_from_slice(&byte);
        let id = usize::from_be_bytes(byte_array);
        match id {
            1 => PacketType::BoundingBoxPacket,
            3 => PacketType::DataChannelPortPacket,
            4 => PacketType::FileBodyPacket,
            5 => PacketType::FileHeaderPacket,
            6 => PacketType::InferenceTypePacket,
            7 => PacketType::StopInferencePacket,
            8 => PacketType::StopInferenceReturnPacket,
            _ => PacketType::BasePacket
        }
    }
}

pub fn length_to_byte(length: usize) -> Vec<u8> {
    length.to_be_bytes().to_vec()
}
