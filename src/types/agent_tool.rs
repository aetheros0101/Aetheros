use async_trait::async_trait;

/// V10 Sprint 3 (Risk Engine): bir tool'un taşıdığı temel risk seviyesi.
/// Şimdilik SADECE tool-bazlı — hangi argümanlarla çağrıldığına
/// bakılmıyor (bkz. security::risk_engine::RiskEngine). Ord derive'ı
/// `High > Medium > Low` karşılaştırmasını doğal kılıyor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

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

    /// Bu tool'un taşıdığı temel risk seviyesi. Varsayılan `Low` —
    /// mevcut tool implementasyonları override etmek zorunda değil.
    /// Riskli tool'lar (WASM çalıştırma, dosya sistemi, ağ, süreç
    /// başlatma vb.) bunu mutlaka override etmeli.
    fn risk_level(
        &self,
    ) -> RiskLevel {
        RiskLevel::Low
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
