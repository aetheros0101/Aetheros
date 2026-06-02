use async_trait::async_trait;

use crate::agents::memory::AgentMemoryRecord;

#[async_trait]
pub trait AgentPersistence:
    Send + Sync
{
    async fn store(
        &self,
        record:
            AgentMemoryRecord,
    );

    async fn load_all(
        &self,
    ) -> Vec<AgentMemoryRecord>;
}
