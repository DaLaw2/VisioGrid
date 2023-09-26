use std::any::Any;
pub trait Packet {
    fn as_any(&self) -> &dyn Any;
    fn get_length_byte(&self) -> Vec<u8>;
    fn get_id_byte(&self) -> Vec<u8>;
    fn get_data_byte(&self) -> Vec<u8>;
    fn get_data_string(&self) -> String;
    fn length_to_byte(length: usize) -> Vec<u8> {
        length.to_be_bytes().to_vec()
    }
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
    pub fn get_id(&self) -> Vec<u8> {
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

    pub fn get_type(byte: Vec<u8>) -> PacketType {
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