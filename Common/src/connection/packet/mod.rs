pub mod base_packet;

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
    AgentInformationPacket,
    AgentInformationAcknowledgePacket,
    AlivePacket,
    AliveAcknowledgePacket,
    ControlPacket,
    ControlAcknowledgePacket,
    DataChannelPortPacket,
    FileBodyPacket,
    FileHeaderPacket,
    FileHeaderAcknowledgePacket,
    FileTransferResultPacket,
    PerformancePacket,
    PerformanceAcknowledgePacket,
    ResultPacket,
    ResultAcknowledgePacket,
    StillProcessPacket,
    StillProcessAcknowledgePacket,
    TaskInfoPacket,
    TaskInfoAcknowledgePacket,
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
            1 => PacketType::AgentInformationPacket,
            2 => PacketType::AgentInformationAcknowledgePacket,
            3 => PacketType::AlivePacket,
            4 => PacketType::AliveAcknowledgePacket,
            5 => PacketType::ControlPacket,
            6 => PacketType::ControlAcknowledgePacket,
            7 => PacketType::DataChannelPortPacket,
            8 => PacketType::FileBodyPacket,
            9 => PacketType::FileHeaderPacket,
            10 => PacketType::FileHeaderAcknowledgePacket,
            11 => PacketType::FileTransferResultPacket,
            12 => PacketType::PerformancePacket,
            13 => PacketType::PerformanceAcknowledgePacket,
            14 => PacketType::ResultPacket,
            15 => PacketType::ResultAcknowledgePacket,
            16 => PacketType::StillProcessPacket,
            17 => PacketType::StillProcessAcknowledgePacket,
            18 => PacketType::TaskInfoPacket,
            19 => PacketType::TaskInfoAcknowledgePacket,
            _ => PacketType::BasePacket,
        }
    }
}

pub fn length_to_byte(length: usize) -> Vec<u8> {
    length.to_be_bytes().to_vec()
}
