use std::collections::BinaryHeap;

use serde::{
    Deserialize,
    Serialize,
};

use uuid::Uuid;

use crate::workflows::scheduler::WorkflowPriority;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
)]
pub struct QueuedWorkflow {
    pub workflow_id:
        Uuid,

    pub priority:
        WorkflowPriority,
}

impl Ord for QueuedWorkflow {
    fn cmp(
        &self,
        other: &Self,
    ) -> std::cmp::Ordering {
        self.priority
            .cmp(
                &other.priority,
            )
    }
}

impl PartialOrd
    for QueuedWorkflow
{
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<
        std::cmp::Ordering,
    > {
        Some(
            self.cmp(other),
        )
    }
}

pub struct WorkflowQueue {
    heap:
        BinaryHeap<
            QueuedWorkflow,
        >,
}

impl WorkflowQueue {
    pub fn new() -> Self {
        Self {
            heap:
                BinaryHeap::new(),
        }
    }

    pub fn push(
        &mut self,
        workflow:
            QueuedWorkflow,
    ) {
        self.heap.push(
            workflow,
        );
    }

    pub fn pop(
        &mut self,
    ) -> Option<
        QueuedWorkflow,
    > {
        self.heap.pop()
    }
}
