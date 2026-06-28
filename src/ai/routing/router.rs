// ============================================================
// src/ai/routing/router.rs
//
// ProviderRouter: birden çok AI provider'ı (Anthropic, OpenAI,
// Gemini, Ollama) provider_id ile kayıt altına alan registry +
// "aktif" provider seçimi.
//
// TASARIM KARARI (Haziran 2026):
//   Otomatik öncelik sıralı fallback ZİNCİRİ yok. Birden fazla
//   provider aktif/yapılandırılmışsa hangisinin kullanılacağına
//   kullanıcı karar verir (Ayarlar ekranında "Aktif Model" seçimi).
//   Router'ın görevi sadece "şu an aktif olan provider'ı çağır".
//
//   Bunun nedeni: kullanıcı API key girerek bir modeli aktif eder
//   (cloud — Anthropic/OpenAI, ücretsiz — Gemini, ya da kendi
//   indirdiği local model — Ollama). Hangisini ne zaman kullanacağı
//   maliyet/gizlilik/kalite tercihi olduğu için otomatik bir
//   sıralama kullanıcının iradesini geçersiz kılardı.
// ============================================================

use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::RwLock;

use crate::ai::errors::AiError;
use crate::ai::inference::request::InferenceRequest;
use crate::ai::inference::response::InferenceResponse;
use crate::ai::providers::anthropic::AnthropicProvider;
use crate::ai::providers::gemini::GeminiProvider;
use crate::ai::providers::ollama::OllamaProvider;
use crate::ai::providers::openai::OpenAiProvider;
use crate::ai::providers::provider::ModelProvider;

pub struct ProviderRouter {
    providers: DashMap<String, Arc<dyn ModelProvider>>,
    active: RwLock<Option<String>>,
}

impl ProviderRouter {
    pub fn new() -> Self {
        Self {
            providers: DashMap::new(),
            active: RwLock::new(None),
        }
    }

    /// Provider kaydet/güncelle (örn. aynı id ile yeni key girilirse
    /// önceki provider'ın üzerine yazılır).
    ///
    /// İlk kayıt edilen provider otomatik olarak aktif yapılır
    /// (henüz hiçbir provider aktif değilse) — kullanıcı tek bir
    /// model bağladığında ek bir "aktif et" adımına gerek kalmaz.
    pub fn register(&self, provider: Arc<dyn ModelProvider>) {
        let id = provider.provider_id().to_string();
        self.providers.insert(id.clone(), provider);

        let mut active = self.active.write();
        if active.is_none() {
            *active = Some(id);
        }
    }

    /// Provider'ı registry'den çıkar (kullanıcı API key'i / bağlantıyı
    /// kaldırdığında). Çıkarılan provider aktifse aktif seçim temizlenir.
    pub fn unregister(&self, provider_id: &str) {
        self.providers.remove(provider_id);

        let mut active = self.active.write();
        if active.as_deref() == Some(provider_id) {
            *active = None;
        }
    }

    pub fn provider(&self, provider_id: &str) -> Option<Arc<dyn ModelProvider>> {
        self.providers
            .get(provider_id)
            .map(|entry| Arc::clone(entry.value()))
    }

    /// Kayıtlı tüm provider id'leri.
    pub fn registered_ids(&self) -> Vec<String> {
        self.providers.iter().map(|e| e.key().clone()).collect()
    }

    /// Aktif provider'ı kullanıcı seçimine göre ayarla.
    /// Provider henüz kayıtlı değilse ProviderUnavailable döner.
    pub fn set_active(&self, provider_id: &str) -> Result<(), AiError> {
        if !self.providers.contains_key(provider_id) {
            return Err(AiError::ProviderUnavailable);
        }
        *self.active.write() = Some(provider_id.to_string());
        Ok(())
    }

    pub fn active_id(&self) -> Option<String> {
        self.active.read().clone()
    }

    /// Şu an aktif olan provider üzerinden inference çalıştır.
    /// Hiç provider aktif/kayıtlı değilse ProviderUnavailable döner.
    pub async fn infer_active(
        &self,
        request: InferenceRequest,
    ) -> Result<InferenceResponse, AiError> {
        let active_id = self.active_id().ok_or(AiError::ProviderUnavailable)?;
        let provider = self
            .provider(&active_id)
            .ok_or(AiError::ProviderUnavailable)?;
        provider.infer(request).await
    }
}

impl Default for ProviderRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// provider_id + kullanıcı girdilerinden somut bir ModelProvider inşa et.
///
/// Hem mobil bridge (Flutter Settings → configure_ai_provider) hem de
/// standalone server (main.rs, env var'lardan) tarafından kullanılır.
///
/// - "anthropic" / "openai" / "gemini" → api_key zorunlu
/// - "ollama"                          → api_key gerekmez; base_url
///   verilmezse http://127.0.0.1:11434, model verilmezse "llama3.2"
pub fn build_provider(
    provider_id: &str,
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<Arc<dyn ModelProvider>, AiError> {
    match provider_id {
        "anthropic" => {
            let key = api_key.ok_or(AiError::ProviderUnavailable)?;
            let mut provider = AnthropicProvider::from_key(key);
            if let Some(m) = model {
                provider = provider.with_model(m);
            }
            Ok(Arc::new(provider))
        }
        "openai" => {
            let key = api_key.ok_or(AiError::ProviderUnavailable)?;
            let mut provider = OpenAiProvider::from_key(key);
            if let Some(m) = model {
                provider = provider.with_model(m);
            }
            Ok(Arc::new(provider))
        }
        "gemini" => {
            let key = api_key.ok_or(AiError::ProviderUnavailable)?;
            let mut provider = GeminiProvider::new(key);
            if let Some(m) = model {
                provider = provider.with_model(m);
            }
            Ok(Arc::new(provider))
        }
        "ollama" => {
            let host = base_url.unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
            let model = model.unwrap_or_else(|| "llama3.2".to_string());
            Ok(Arc::new(OllamaProvider::with_host(host, model)))
        }
        other => Err(AiError::ProviderFailure {
            message: format!("Bilinmeyen provider_id: {other}"),
        }),
    }
}
