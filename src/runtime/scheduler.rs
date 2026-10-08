use std::sync::Arc;

use crate::task::queue::PriorityTaskQueue;
use crate::task::task::TaskDefinition;

pub struct Scheduler {
    queue: Arc<PriorityTaskQueue>,
}

impl Scheduler {
    pub fn new(queue: Arc<PriorityTaskQueue>) -> Self {
        Self { queue }
    }

    pub async fn submit(&self, task: TaskDefinition) {
        let _ = self.queue.push(task).await;
    }
}
