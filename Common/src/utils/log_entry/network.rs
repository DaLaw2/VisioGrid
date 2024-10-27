use std::io::Error as IOError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetworkEntry {
    #[error("Channel has been closed")]
    ChannelClosed,
    #[error("Control channel timeout")]
    ControlChannelTimeout,
    #[error("Create data channel success")]
    CreateDataChannelSuccess,
    #[error("Create data channel timeout")]
    CreateDataChannelTimout,
    #[error("Data channel timeout")]
    DataChannelTimeout,
    #[error("Data channel not ready")]
    DataChannelNotReady,
    #[error("Failed to bind port: {0}")]
    BindPortError(IOError),
    #[error("Failed to establish connection: {0}")]
    EstablishConnectionError(IOError),
    #[error("Receive unexpected packet")]
    UnexpectedPacket,
    #[error("Agent side disconnect")]
    AgentDisconnect,
    #[error("Management side disconnect")]
    ManagementDisconnect,
    #[error("Failed to destroy instance")]
    DestroyInstanceError,
}

impl From<NetworkEntry> for String {
    #[inline(always)]
    fn from(value: NetworkEntry) -> Self {
        value.to_string()
    }
}
