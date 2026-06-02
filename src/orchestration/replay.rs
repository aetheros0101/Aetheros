use std::collections::HashSet;

use uuid::Uuid;

use crate::orchestration::graph::{
    ExecutionGraph,
    ExecutionNode,
};
use crate::orchestration::events::OrchestrationEvent;

pub struct ReplaySession {
    pub execution_id:
        Uuid,

    pub events:
        Vec<OrchestrationEvent>,
}

impl ReplaySession {
    pub fn new(
        execution_id: Uuid,
    ) -> Self {
        Self {
            execution_id,

            events: Vec::new(),
        }
    }

    pub fn append(
        &mut self,
        event: OrchestrationEvent,
    ) {
        self.events.push(event);
    }

    pub fn event_count(
        &self,
    ) -> usize {
        self.events.len()
    }
}

pub struct ReplayState {
    pub completed:
        HashSet<Uuid>,
}

pub struct ReplayEngine;

impl ReplayEngine {
    pub fn remaining_nodes(
        graph: &ExecutionGraph,

        replay:
            &ReplayState,
    ) -> Vec<ExecutionNode> {
        graph
            .nodes
            .iter()
            .filter(|node| {
                !replay
                    .completed
                    .contains(
                        &node.id,
                    )
            })
            .cloned()
            .collect()
    }
}
