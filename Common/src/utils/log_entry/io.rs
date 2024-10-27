use serde_json::error::Error as SerdeJsonError;
use std::io::Error as IoError;
use std::path::Display;
use thiserror::Error;
use toml::ser::Error as TomlError;

#[derive(Error, Debug)]
pub enum IOEntry<'a> {
    #[error("Failed to create directory {0}: {1}")]
    CreateDirectoryError(Display<'a>, IoError),
    #[error("Failed to create file {0}: {1}")]
    CreateFileError(Display<'a>, IoError),
    #[error("Failed to delete directory {0}: {1}")]
    DeleteDirectoryError(Display<'a>, IoError),
    #[error("Failed to delete file {0}: {1}")]
    DeleteFileError(Display<'a>, IoError),
    #[error("Failed to move directory {0} to {1}: {2}")]
    MoveDirectoryError(Display<'a>, Display<'a>, IoError),
    #[error("Failed to move file {0} to {1}: {2}")]
    MoveFileError(Display<'a>, Display<'a>, IoError),
    #[error("Failed to read directory {0}: {1}")]
    ReadDirectoryError(Display<'a>, IoError),
    #[error("Failed to read file {0}: {1}")]
    ReadFileError(Display<'a>, IoError),
    #[error("Failed to write directory {0}: {1}")]
    WriteDirectoryError(Display<'a>, IoError),
    #[error("Failed to write file {0}: {1}")]
    WriteFileError(Display<'a>, IoError),
    #[error("Failed to get absolute path of file {0}: {1}")]
    GetAbsolutePathError(Display<'a>, IoError),
    #[error("Failed to serialize to TOML: {0}")]
    TomlSerializeError(TomlError),
    #[error("Failed to serialize data: {0}")]
    SerdeSerializeError(SerdeJsonError),
    #[error("Failed to deserialize data: {0}")]
    SerdeDeserializeError(SerdeJsonError),
}

impl From<IOEntry<'_>> for String {
    #[inline(always)]
    fn from(value: IOEntry) -> Self {
        value.to_string()
    }
}
