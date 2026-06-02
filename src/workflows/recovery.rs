use crate::workflows::checkpoint::WorkflowCheckpoint;
use uuid::Uuid;

pub struct WorkflowRecovery;

impl WorkflowRecovery {
    pub fn resumable(
        checkpoint:
            &WorkflowCheckpoint,
    ) -> bool {
        !checkpoint
            .pending_nodes
            .is_empty()
    }

    pub async fn recover(
        checkpoint:
            WorkflowCheckpoint,
    ) {
        let _ =
            checkpoint;
    }
}
pub struct WorkflowRecoveryPlan {
    pub workflow_id:
        Uuid,

    pub checkpoint_id:
        Uuid,

    pub replay_events:
        bool,
}
