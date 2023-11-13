pub trait Packet: Send {
    fn as_length_byte(&self) -> &[u8];
    fn as_id_byte(&self) -> &[u8];
    fn as_data_byte(&self) -> &[u8];
    fn clone_length_byte(&self) -> Vec<u8>;
    fn clone_id_byte(&self) -> Vec<u8>;
    fn clone_data_byte(&self) -> Vec<u8>;
    fn data_to_string(&self) -> String;
    fn packet_type(&self) -> PacketType;
    fn equal(&self, packet_type: PacketType) -> bool;
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum PacketType {
    BasePacket,
    AlivePacket,
    AliveReplyPacket,
    ConfirmPacket,
    ControlPacket,
    ControlReplyPacket,
    PerformancePacket,
    DataChannelPortPacket,
    TaskInfoPacket,
    TaskInfoReplyPacket,
    FileHeaderPacket,
    FileBodyPacket,
    FileTransferReplyPacket,
    ResultPacket,
}

impl PacketType {
    pub fn as_id_byte(&self) -> Vec<u8> {
        let id: usize = match self {
            PacketType::BasePacket => 0,
            PacketType::AlivePacket => 1,
            PacketType::AliveReplyPacket => 2,
            PacketType::ConfirmPacket => 3,
            PacketType::ControlPacket => 4,
            PacketType::ControlReplyPacket => 5,
            PacketType::PerformancePacket => 6,
            PacketType::DataChannelPortPacket => 7,
            PacketType::TaskInfoPacket => 8,
            PacketType::TaskInfoReplyPacket => 9,
            PacketType::FileHeaderPacket => 10,
            PacketType::FileBodyPacket => 11,
            PacketType::FileTransferReplyPacket => 12,
            PacketType::ResultPacket => 13,
        };
        id.to_be_bytes().to_vec()
    }

    pub fn parse_packet_type(byte: &Vec<u8>) -> PacketType {
        let mut byte_array = [0_u8; 8];
        byte_array.copy_from_slice(&byte);
        let id = usize::from_be_bytes(byte_array);
        match id {
            1 => PacketType::AlivePacket,
            2 => PacketType::AliveReplyPacket,
            3 => PacketType::ConfirmPacket,
            4 => PacketType::ControlPacket,
            5 => PacketType::ControlReplyPacket,
            6 => PacketType::PerformancePacket,
            7 => PacketType::DataChannelPortPacket,
            8 => PacketType::TaskInfoPacket,
            9 => PacketType::TaskInfoReplyPacket,
            10 => PacketType::FileHeaderPacket,
            11 => PacketType::FileBodyPacket,
            12 => PacketType::FileTransferReplyPacket,
            13 => PacketType::ResultPacket,
            _ => PacketType::BasePacket
        }
    }
}

pub fn length_to_byte(length: usize) -> Vec<u8> {
    length.to_be_bytes().to_vec()
}
