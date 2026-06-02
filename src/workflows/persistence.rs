use async_trait::async_trait;

use uuid::Uuid;

use crate::workflows::state::WorkflowState;

#[async_trait]
pub trait WorkflowPersistence:
    Send + Sync
{
    async fn persist_state(
        &self,
        workflow_id:
            Uuid,

        state:
            WorkflowState,
    );

    async fn load_state(
        &self,
        workflow_id:
            Uuid,
    ) -> Option<
        WorkflowState,
    >;
}
