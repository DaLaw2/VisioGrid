pub trait Packet {
    fn get_length(&self) -> Vec<u8>;
    fn get_id(&self) -> Vec<u8>;
    fn get_data(&self) -> Vec<u8>;
    fn
    fn equal(&self, packet_type: PacketType) -> bool;
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum PacketType {
    Empty,
    PicturePacket,
    DataChannelPortPacket,
    InferenceTypePacket,
    BoundingBoxSizePacket,
    BoundingBoxPacket,
    StopInferencePacket,
    StopInferenceReturnPacket
}

impl PacketType {
    pub fn get_id(&self) -> Vec<u8> {
        let id: usize = match self {
            PacketType::Empty => 0,
            PacketType::PicturePacket => 1,
            PacketType::DataChannelPortPacket => 2,
            PacketType::InferenceTypePacket => 3,
            PacketType::BoundingBoxSizePacket => 4,
            PacketType::BoundingBoxPacket => 5,
            PacketType::StopInferencePacket => 6,
            PacketType::StopInferenceReturnPacket => 7,
        };
        vec![(id / 10) as u8 + 48, (id % 10) as u8 + 48]
    }

    pub fn get_type(byte: Vec<u8>) -> PacketType {
        let mut id = 0_usize;
        for &digit in byte.iter() {
            id = id * 10 + (digit - 48) as usize;
        }
        match id {
            0 => PacketType::Empty,
            1 => PacketType::PicturePacket,
            2 => PacketType::DataChannelPortPacket,
            3 => PacketType::InferenceTypePacket,
            4 => PacketType::BoundingBoxSizePacket,
            5 => PacketType::BoundingBoxPacket,
            6 => PacketType::StopInferencePacket,
            7 => PacketType::StopInferenceReturnPacket,
            _ => PacketType::Empty
        }
    }
}