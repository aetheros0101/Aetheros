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

    /// Bu tool'un çalışması için agent'ın hangi capability'ye sahip
    /// olması gerekiyor. Varsayılan `None` — mevcut tool implementasyonları
    /// bunu override etmek zorunda değil (geriye dönük uyumluluk).
    /// Riskli tool'lar (dosya sistemi, ağ, süreç çalıştırma vb.)
    /// bunu mutlaka override etmeli.
    fn required_capability(
        &self,
    ) -> Option<crate::agents::capabilities::AgentCapability> {
        None
    }

    async fn invoke(
        &self,
        arguments:
            Vec<String>,
    ) -> Result<
        String,
        String,
    >;
}
