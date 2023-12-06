use crate::connection::packet::definition::Packet;
use crate::connection::packet::base_packet::BasePacket;

pub struct FileTransferResult {
    pub result: Option<Vec<usize>>,
}

impl FileTransferResult {
    pub fn into(self) -> Option<Vec<usize>> {
        self.result
    }

    pub fn parse_from_packet(file_transfer_reply_packet: &BasePacket) -> Self {
        let data = file_transfer_reply_packet.as_data_byte();
        if data.is_empty() {
            Self {
                result: None,
            }
        } else {
            let mut result = Vec::new();
            for chunk in data.chunks_exact(8) {
                if let Ok(bytes) = chunk.try_into() {
                    let num = usize::from_be_bytes(bytes);
                    result.push(num);
                }
            }
            Self {
                result: Some(result),
            }
        }
    }
}
