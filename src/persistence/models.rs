use serde::{
    Deserialize,
    Serialize,
};

use crate::task::task::{
    TaskDefinition,
    TaskState,
};
use crate::types::timestamps::Timestamp;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct PersistedTask {
    pub task: TaskDefinition,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub attempts: u32,
}

impl PersistedTask {
    pub fn is_terminal(
        &self,
    ) -> bool {
        matches!(
            self.task.state,
            TaskState::Completed
                | TaskState::Failed
                | TaskState::Cancelled
        )
    }
}
