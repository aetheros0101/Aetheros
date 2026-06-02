use dashmap::DashMap;

use crate::task::task::TaskDefinition;
use crate::types::ids::TaskId;

pub struct TaskRegistry {
    tasks: DashMap<TaskId, TaskDefinition>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            tasks: DashMap::new(),
        }
    }

    pub fn insert(&self, task: TaskDefinition) {
        self.tasks.insert(task.id, task);
    }
}
