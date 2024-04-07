use std::cmp::Ordering;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub enum AgentState {
    None,
    ProcessTask,
    Idle(u64),
    CreateDataChannel,
    Terminate,
}

impl PartialOrd for AgentState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AgentState {
    fn cmp(&self, other: &Self) -> Ordering {
        self.order().cmp(&other.order())
    }
}

impl AgentState {
    fn order(&self) -> u32 {
        match self {
            AgentState::Terminate => 4,
            AgentState::CreateDataChannel => 3,
            AgentState::ProcessTask | AgentState::Idle(_) => 2,
            AgentState::None => 1,
        }
    }
}
