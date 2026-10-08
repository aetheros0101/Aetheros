use serde::{Deserialize, Serialize};

use crate::orchestration::state::OrchestrationState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: OrchestrationState,

    pub to: OrchestrationState,
}

pub struct OrchestrationStateMachine;

impl OrchestrationStateMachine {
    pub fn can_transition(from: OrchestrationState, to: OrchestrationState) -> bool {
        matches!(
            (from, to),
            (OrchestrationState::Pending, OrchestrationState::Running,)
                | (OrchestrationState::Running, OrchestrationState::Completed,)
                | (OrchestrationState::Running, OrchestrationState::Failed,)
                | (OrchestrationState::Failed, OrchestrationState::Retrying,)
                | (OrchestrationState::Retrying, OrchestrationState::Running,)
        )
    }
}
