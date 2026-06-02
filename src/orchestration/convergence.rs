use std::collections::HashMap;

use uuid::Uuid;

use crate::orchestration::state::OrchestrationState;

pub struct StateConvergence {
    states:
        HashMap<
            Uuid,
            OrchestrationState,
        >,
}

impl StateConvergence {
    pub fn new() -> Self {
        Self {
            states:
                HashMap::new(),
        }
    }

    pub fn update(
        &mut self,
        execution_id:
            Uuid,

        state:
            OrchestrationState,
    ) {
        self.states.insert(
            execution_id,
            state,
        );
    }

    pub fn converged(
        &self,
        execution_id:
            &Uuid,
    ) -> bool {
        self.states
            .contains_key(
                execution_id,
            )
    }
}
