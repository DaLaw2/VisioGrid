use std::io::Error as IoError;
use std::net::SocketAddr;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum SystemEntry {
    #[error("Online now")]
    Online,
    #[error("Initializing")]
    Initializing,
    #[error("Initialization completed")]
    InitializeComplete,
    #[error("Termination in process")]
    Terminating,
    #[error("Termination completed")]
    TerminateComplete,
    #[error("Cleaning up")]
    Cleaning,
    #[error("Cleanup completed")]
    CleanComplete,
    #[error("Operation cancel")]
    Cancel,
    #[error("Invalid configuration")]
    InvalidConfig,
    #[error("Configuration not found")]
    ConfigNotFound,
    #[error("Web service ready")]
    WebReady,
    #[error("Web service panic: {0}")]
    WebPanic(IoError),
    #[error("Management {0} is connected")]
    ManagementConnect(SocketAddr),
    #[error("Agent {0} is connected")]
    AgentConnect(SocketAddr),
    #[error("Agent instance already exists")]
    AgentExistError,
    #[error("Agent instance does not exist")]
    AgentDoesNotExistError,
    #[error("Port pool has no available port")]
    NoAvailablePort,
    #[error("Child process execution error: {0}")]
    ChildProcessError(String),
    #[error("Task panic while execution: {0}")]
    TaskPanickedError(JoinError),
}

impl From<SystemEntry> for String {
    #[inline(always)]
    fn from(value: SystemEntry) -> Self {
        value.to_string()
    }
}
