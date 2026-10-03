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

/// Argüman-bazlı karar (B4). Bir tool, kendi çağrısını ARGÜMANLARIYLA
/// değerlendirip bunu döndürebilir; SecurityGovernor tool-bazlı sabit
/// riskin yerine bunu kullanır. Öncelik: Deny > Allow > Ask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallVerdict {
    /// Açık bir allow kuralı var: onay istemeden çalışır.
    Allow,
    /// İnsan onayı gerekir (varsayılan).
    Ask { reason: String },
    /// Kesin ret — onayla bile çalışmaz.
    Deny { reason: String },
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

    /// V10 Faz 1: planner'ın AI'ye gösterdiği kısa kullanım açıklaması
    /// (özellikle ARGÜMAN SÖZLEŞMESİ). Varsayılan boş — mevcut tool'lar
    /// override etmek zorunda değil. Olmadan AI, `arguments` alanını nasıl
    /// dolduracağını tahmin etmek zorunda kalır.
    fn description(
        &self,
    ) -> &'static str {
        ""
    }

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

    /// B4: bu tool'un BU argümanlarla çağrılmasına dair karar. Varsayılan
    /// `None` = "argümana bakmıyorum" → Governor eski davranışı (tool-bazlı
    /// sabit risk) uygular. Riskli tool'lar (terminal) override etmeli.
    fn assess_call(
        &self,
        _arguments: &[String],
    ) -> Option<CallVerdict> {
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
