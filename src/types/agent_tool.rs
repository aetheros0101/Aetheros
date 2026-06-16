use async_trait::async_trait;

/// `agents` ve `ai` modüllerinin her ikisi de bu trait'e ihtiyaç duyduğu için
/// döngüsel bağımlılığı önlemek amacıyla buraya (types) taşındı.
/// Önceki konum: src/agents/tools.rs
#[async_trait]
pub trait AgentTool:
    Send + Sync
{
    fn name(
        &self,
    ) -> &'static str;

    async fn invoke(
        &self,
        arguments:
            Vec<String>,
    ) -> Result<
        String,
        String,
    >;
}
