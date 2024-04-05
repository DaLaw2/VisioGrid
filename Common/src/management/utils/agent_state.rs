use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub enum AgentState {
    None,
    ProcessTask,
    Idle(u64),
    CreateDataChannel,
    Terminate,
}
