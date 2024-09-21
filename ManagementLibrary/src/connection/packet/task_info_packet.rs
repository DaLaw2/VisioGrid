use crate::connection::packet::{length_to_byte, Packet, PacketType};
use crate::utils::DefinePacketWithData;

#[derive(DefinePacketWithData)]
pub struct TaskInfoPacket {
    length: Vec<u8>,
    id: Vec<u8>,
    data: Vec<u8>,
    packet_type: PacketType,
}
