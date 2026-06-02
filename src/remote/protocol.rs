use serde::{
    Deserialize,
    Serialize,
};

use uuid::Uuid;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub enum RemoteCommand {
    ExecuteTask {
        task_id: Uuid,
    },

    ExecuteWorkflow {
        workflow_id: Uuid,
    },

    CancelExecution {
        execution_id: Uuid,
    },

    ReplayExecution {
        execution_id: Uuid,
    },

    SynchronizeState {
        execution_id: Uuid,
    },

    TransferOwnership {
        execution_id: Uuid,

        target_node: Uuid,
    },

    Heartbeat,
}
