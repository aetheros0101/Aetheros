use serde::{
    Deserialize,
    Serialize,
};

use crate::types::ids::{
    ExecutionId,
    TaskId,
};

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct TaskContext {
    pub task_id: TaskId,
    pub execution_id: ExecutionId,
    pub attempt: u32,
}
