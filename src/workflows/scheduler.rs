use std::collections::{
    HashMap,
    HashSet,
};

use serde::{
    Deserialize,
    Serialize,
};

use uuid::Uuid;

use crate::workflows::execution_graph::{
    ExecutionNode,
    WorkflowExecutionGraph,
};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
pub enum WorkflowPriority {
    Low,
    Normal,
    High,
    Critical,
}

pub struct WorkflowScheduler;

impl WorkflowScheduler {
    pub fn ready_nodes(
        graph:
            &WorkflowExecutionGraph,

        completed:
            &HashSet<Uuid>,
    ) -> Vec<Uuid> {
        let node_map:
            HashMap<
                Uuid,
                &ExecutionNode,
            > = graph
            .nodes
            .iter()
            .map(|node| {
                (node.id, node)
            })
            .collect();

        let mut ready =
            Vec::new();

        for node in node_map.values() {
            let dependencies_met =
                node
                    .dependencies
                    .iter()
                    .all(|dependency| {
                        completed.contains(
                            dependency,
                        )
                    });

            if dependencies_met
                && !completed.contains(
                    &node.id,
                )
            {
                ready.push(node.id);
            }
        }

        ready
    }
}
