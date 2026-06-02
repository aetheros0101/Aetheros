use crate::agents::state::AgentState;

use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
)]
pub enum AgentLifecycleState {
    Initializing,
    Planning,
    Executing,
    Waiting,
    Retrying,
    Suspended,
    Failed,
    Completed,
}

pub struct AgentLifecycle {
    state: AgentState,
}

impl AgentLifecycle {
    pub fn new() -> Self {
        Self {
            state:
                AgentState::Registered,
        }
    }

    pub fn transition(
        &mut self,
        state: AgentState,
    ) {
        self.state = state;
    }

    pub fn state(
        &self,
    ) -> AgentState {
        self.state
    }
}
