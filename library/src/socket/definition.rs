pub trait Packet {
    fn get_length_byte(&self) -> Vec<u8>;
    fn get_id_byte(&self) -> Vec<u8>;
    fn get_data_byte(&self) -> Vec<u8>;
    fn get_data_string(&self) -> String;
    fn length_to_byte(length: usize) -> Vec<u8> {
        let mut byte: Vec<u8> = Vec::new();
        for digital in (0..8).rev() {
            byte.push(((length >> (digital * 8)) & 0xFF) as u8);
        }
        byte
    }
    fn get_info(&self) -> String;
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
        vec![id as u8]
    }

    pub fn get_type(byte: Vec<u8>) -> PacketType {
        let mut id = 0_usize;
        for &digit in byte.iter() {
            id = id * 10 + digit as usize;
        }
        match id {
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