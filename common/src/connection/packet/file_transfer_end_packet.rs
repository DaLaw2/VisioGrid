use crate::connection::packet::{length_to_byte, Packet, PacketType};
use crate::utils::DefinePacketWithoutData;

#[derive(DefinePacketWithoutData)]
pub struct FileTransferEndPacket {
    length: Vec<u8>,
    id: Vec<u8>,
    data: Vec<u8>,
    packet_type: PacketType,
}
