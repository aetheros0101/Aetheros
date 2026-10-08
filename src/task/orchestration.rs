use serde::{Deserialize, Serialize};

use crate::types::ids::TaskId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependency {
    pub depends_on: TaskId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOrchestration {
    pub dependencies: Vec<TaskDependency>,
}
