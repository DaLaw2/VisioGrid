use thiserror::Error;

#[derive(Error, Debug)]
pub enum MiscEntry {
    #[error("Invalid file name")]
    InvalidFileNameError,
    #[error("Missing file block")]
    MissingFileBlockError,
    #[error("Received invalid packet")]
    InvalidPacket,
    #[error("Wrong packet deliver order")]
    WrongDeliverOrder,
}

impl From<MiscEntry> for String {
    #[inline(always)]
    fn from(value: MiscEntry) -> Self {
        value.to_string()
    }
}
