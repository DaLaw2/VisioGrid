use crate::connection::packet::base_packet::BasePacket;
use crate::connection::packet::{Packet, PacketType, length_to_byte};

pub struct StillProcessReplyPacket {
    length: Vec<u8>,
    id: Vec<u8>,
    data: Vec<u8>,
    packet_type: PacketType,
}

impl StillProcessReplyPacket {
    pub fn new() -> Self {
        Self {
            length: length_to_byte(16),
            id: PacketType::StillProcessReplyPacket.as_id_byte(),
            data: Vec::new(),
            packet_type: PacketType::StillProcessReplyPacket
        }
    }

    pub fn from_base_packet(base_packet: BasePacket) -> Self {
        Self {
            length: base_packet.length,
            id: base_packet.id,
            data: base_packet.data,
            packet_type: PacketType::StillProcessReplyPacket
        }
    }
}

impl Packet for StillProcessReplyPacket {
    fn as_length_byte(&self) -> &[u8] {
        &self.length
    }

    fn as_id_byte(&self) -> &[u8] {
        &self.id
    }

    fn as_data_byte(&self) -> &[u8] {
        &self.data
    }

    fn clone_length_byte(&self) -> Vec<u8> {
        self.length.clone()
    }

    fn clone_id_byte(&self) -> Vec<u8> {
        self.id.clone()
    }

    fn clone_data_byte(&self) -> Vec<u8> {
        self.data.clone()
    }

    fn data_to_string(&self) -> String {
        String::from_utf8_lossy(&*self.data.clone()).to_string()
    }

    fn packet_type(&self) -> PacketType {
        self.packet_type
    }

    fn equal(&self, packet_type: PacketType) -> bool {
        self.packet_type.eq(&packet_type)
    }
}
