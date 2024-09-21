use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum TaskEntry {
    #[error("Task {0}, unsupported file type")]
    UnSupportFileType(Uuid),
    #[error("Task {0} cannot be assigned to any agent")]
    TaskAssignError(Uuid),
    #[error("Task {0} does not exist")]
    TaskDoesNotExist(Uuid),
    #[error("Error occur while agent processing task: {0}")]
    AgentProcessingError(String),
}

impl From<TaskEntry> for String {
    #[inline(always)]
    fn from(value: TaskEntry) -> Self {
        value.to_string()
    }
}
