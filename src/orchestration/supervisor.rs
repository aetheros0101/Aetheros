use std::collections::HashMap;

use uuid::Uuid;

use crate::orchestration::state::OrchestrationState;

pub struct RuntimeSupervisor {
    executions:
        HashMap<
            Uuid,
            OrchestrationState,
        >,
}

impl RuntimeSupervisor {
    pub fn new() -> Self {
        Self {
            executions:
                HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        id: Uuid,
    ) {
        self.executions.insert(
            id,
            OrchestrationState::Pending,
        );
    }

    pub fn update(
        &mut self,
        id: Uuid,

        state:
            OrchestrationState,
    ) {
        self.executions.insert(
            id,
            state,
        );
    }

    pub fn state(
        &self,
        id: &Uuid,
    ) -> Option<
        OrchestrationState,
    > {
        self.executions
            .get(id)
            .copied()
    }
}
