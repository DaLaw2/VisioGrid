#[derive(Eq, PartialEq, Clone, Copy)]
pub enum ConfirmType {
    ReceiveAgentInformationSuccess,
    ReceivePerformanceSuccess,
}

impl ConfirmType {
    pub fn as_byte(&self) -> Vec<u8> {
        let id: usize = *self as usize;
        id.to_be_bytes().to_vec()
    }
}
