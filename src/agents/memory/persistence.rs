use async_trait::async_trait;
use crate::agents::memory::AgentMemoryRecord;

#[async_trait]
pub trait AgentPersistence: Send + Sync {
    async fn store(&self, record: AgentMemoryRecord);
    async fn load_all(&self) -> Vec<AgentMemoryRecord>;
    async fn load_by_prefix(&self, prefix: &str) -> Vec<AgentMemoryRecord> {
        self.load_all()
            .await
            .into_iter()
            .filter(|r| r.key.starts_with(prefix))
            .collect()
    }
}
