use async_trait::async_trait;

use uuid::Uuid;

use crate::errors::runtime::RuntimeError;

use crate::orchestration::graph::ExecutionGraph;

#[async_trait]
pub trait GraphPersistence:
    Send + Sync
{
    async fn save(
        &self,
        graph:
            ExecutionGraph,
    ) -> Result<(), RuntimeError>;

    async fn load(
        &self,
        id: Uuid,
    ) -> Result<
        ExecutionGraph,
        RuntimeError,
    >;
}
