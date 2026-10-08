pub mod deadline;
pub mod lease;
pub mod orchestration;
pub mod priority;
pub mod queue;
pub mod retry;
// `runtime::runtime`-tarzı adlandırma bilinçli (genel API yolu); yeniden adlandırma Faz 1/2.
#[allow(clippy::module_inception)]
pub mod task;

use async_trait::async_trait;

use crate::errors::task::TaskError;
use crate::task::task::TaskDefinition;

#[async_trait]
pub trait TaskQueue {
    async fn enqueue(&self, task: TaskDefinition) -> Result<(), TaskError>;

    async fn dequeue(&self) -> Result<Option<TaskDefinition>, TaskError>;
}
