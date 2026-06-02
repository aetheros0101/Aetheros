use serde::{
    Deserialize,
    Serialize,
};

use crate::types::ids::TaskId;
use crate::task::output::TaskOutput;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub enum TaskResult {
    Success {
        task_id: TaskId,
        output: TaskOutput,
    },

    Failure {
        task_id: TaskId,
        error: TaskFailure,
    },

    Cancelled {
        task_id: TaskId,
    },
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub enum TaskFailure {
    Cancelled,
    DeadlineExceeded,
    RetryExhausted,
    ExecutionFailure,
    PersistenceFailure,
}
