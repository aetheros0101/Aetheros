use async_trait::async_trait;

use crate::ai::memory::embedding::EmbeddingVector;

#[async_trait]
pub trait VectorStore:
    Send + Sync
{
    async fn insert(
        &self,
        key: String,

        embedding:
            EmbeddingVector,
    );

    async fn search(
        &self,
        embedding:
            EmbeddingVector,
    ) -> Vec<String>;
}
