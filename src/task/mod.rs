pub mod context;
pub mod deadline;
pub mod lease;
pub mod orchestration;
pub mod output;
pub mod priority;
pub mod queue;
pub mod result;
pub mod retry;
pub mod task;

use async_trait::async_trait;

use crate::errors::task::TaskError;
use crate::task::task::TaskDefinition;

#[async_trait]
pub trait TaskQueue {
    async fn enqueue(&self, task: TaskDefinition) -> Result<(), TaskError>;

    async fn dequeue(&self) -> Result<Option<TaskDefinition>, TaskError>;
}
