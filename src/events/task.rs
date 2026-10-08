use serde::{Deserialize, Serialize};

use crate::types::ids::TaskId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskEvent {
    TaskQueued { task_id: TaskId },

    TaskStarted { task_id: TaskId },

    TaskCompleted { task_id: TaskId },

    TaskFailed { task_id: TaskId },

    TaskCancelled { task_id: TaskId },

    TaskRetrying { task_id: TaskId, attempt: u32 },

    TaskRetried { task_id: TaskId, attempt: u32 },
}
