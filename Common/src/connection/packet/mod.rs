pub mod base_packet;
pub mod file_body_packet;
pub mod file_header_ack_packet;
pub mod file_header_packet;
pub mod file_transfer_end_packet;
pub mod file_transfer_result_packet;

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

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum PacketType {
    BasePacket,
    AgentInfoPacket,
    AgentInfoAckPacket,
    AlivePacket,
    AliveAckPacket,
    ControlPacket,
    ControlAckPacket,
    DataChannelPortPacket,
    FileBodyPacket,
    FileHeaderPacket,
    FileHeaderAckPacket,
    FileTransferResultPacket,
    FileTransferEndPacket,
    PerformancePacket,
    PerformanceAckPacket,
    TaskResultPacket,
    TaskResultAckPacket,
    StillProcessPacket,
    StillProcessAckPacket,
    TaskInfoPacket,
    TaskInfoAckPacket,
}

impl PacketType {
    pub fn as_byte(&self) -> Vec<u8> {
        let id: usize = *self as usize;
        id.to_be_bytes().to_vec()
    }

    pub fn parse_packet_type(byte: &Vec<u8>) -> PacketType {
        let mut byte_array = [0_u8; 8];
        byte_array.copy_from_slice(&byte);
        let id = usize::from_be_bytes(byte_array);
        match id {
            1 => PacketType::AgentInfoPacket,
            2 => PacketType::AgentInfoAckPacket,
            3 => PacketType::AlivePacket,
            4 => PacketType::AliveAckPacket,
            5 => PacketType::ControlPacket,
            6 => PacketType::ControlAckPacket,
            7 => PacketType::DataChannelPortPacket,
            8 => PacketType::FileBodyPacket,
            9 => PacketType::FileHeaderPacket,
            10 => PacketType::FileHeaderAckPacket,
            11 => PacketType::FileTransferResultPacket,
            12 => PacketType::FileTransferEndPacket,
            13 => PacketType::PerformancePacket,
            14 => PacketType::PerformanceAckPacket,
            15 => PacketType::TaskResultPacket,
            16 => PacketType::TaskResultAckPacket,
            17 => PacketType::StillProcessPacket,
            18 => PacketType::StillProcessAckPacket,
            19 => PacketType::TaskInfoPacket,
            20 => PacketType::TaskInfoAckPacket,
            _ => PacketType::BasePacket,
        }
    }
}

pub fn length_to_byte(length: usize) -> Vec<u8> {
    length.to_be_bytes().to_vec()
}
