use tokio::sync::mpsc;

use crate::errors::runtime::RuntimeError;
use crate::task::task::TaskDefinition;

#[derive(Clone)]
pub struct RuntimeHandle {
    sender: mpsc::Sender<TaskDefinition>,
}

impl RuntimeHandle {
    pub fn new(
        sender: mpsc::Sender<TaskDefinition>,
    ) -> Self {
        Self {
            sender,
        }
    }

    pub async fn submit(
        &self,
        task: TaskDefinition,
    ) -> Result<(), RuntimeError>
    {
        self.sender
            .send(task)
            .await
            .map_err(|_| {
                RuntimeError::QueueClosed
            })?;

        Ok(())
    }
}
