use crate::connection::packet::base_packet::BasePacket;
use crate::connection::packet::definition::Packet;

pub struct FileTransferResult {
    pub result: Option<Vec<usize>>,
}

impl FileTransferResult {
    pub fn parse_from_packet(file_transfer_reply_packet: &BasePacket) -> Self {
        let data = file_transfer_reply_packet.as_data_byte();
        if data.is_empty() {
            Self {
                result: None,
            }
        } else {
            let mut result = Vec::new();
            for chunk in data.chunks_exact(8) {
                match chunk.try_into() {
                    Ok(bytes) => {
                        let num = usize::from_be_bytes(bytes);
                        result.push(num);
                    },
                    Err(_) => continue,
                }
            }
            Self {
                result: Some(result),
            }
        }
    }
}
