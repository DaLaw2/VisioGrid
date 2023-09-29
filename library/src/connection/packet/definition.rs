use std::fmt;

pub trait Packet: fmt::Display {
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
    BoundingBoxSizePacket,
    DataChannelPortPacket,
    InferenceTypePacket,
    PicturePacket,
    StopInferencePacket,
    StopInferenceReturnPacket
}

impl PacketType {
    pub fn as_id_byte(&self) -> Vec<u8> {
        let id: usize = match self {
            PacketType::BasePacket => 0,
            PacketType::BoundingBoxPacket => 1,
            PacketType::BoundingBoxSizePacket => 2,
            PacketType::DataChannelPortPacket => 3,
            PacketType::InferenceTypePacket => 4,
            PacketType::PicturePacket => 5,
            PacketType::StopInferencePacket => 6,
            PacketType::StopInferenceReturnPacket => 7,
        };
        vec![(id / 10) as u8, (id % 10) as u8]
    }

    pub fn get_packet_type(byte: &Vec<u8>) -> PacketType {
        let mut id = 0_usize;
        for &digit in byte.iter() {
            id = id * 10 + digit as usize;
        }
        match id {
            1 => PacketType::BoundingBoxPacket,
            2 => PacketType::BoundingBoxSizePacket,
            3 => PacketType::DataChannelPortPacket,
            4 => PacketType::InferenceTypePacket,
            5 => PacketType::PicturePacket,
            6 => PacketType::StopInferencePacket,
            7 => PacketType::StopInferenceReturnPacket,
            _ => PacketType::BasePacket
        }
    }
}

pub fn length_to_byte(length: usize) -> Vec<u8> {
    length.to_be_bytes().to_vec()
}