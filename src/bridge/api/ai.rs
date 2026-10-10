// ============================================================
// bridge/api/ai.rs
//
// AI provider yapılandırma, test ve sohbet.
// ============================================================

use tracing::info;

use crate::bridge::api::error::require_runtime;
use crate::bridge::state::get_runtime;

// ── AI Provider fonksiyonları (FRB) ───────────────────────────
//
// Kullanıcı Ayarlar ekranında bir provider için API key (Anthropic/
// OpenAI/Gemini) ya da host (Ollama) girip "kaydet"e bastığında
// Flutter bu fonksiyonu çağırır. Birden fazla provider kayıtlıysa
// hangisinin kullanılacağına set_active_ai_provider ile kullanıcı
// karar verir — otomatik fallback YOK (bkz. ai::routing::router).

/// Bir AI provider'ı yapılandır ve ProviderRouter'a kaydet.
///
/// provider_id: "anthropic" | "openai" | "gemini" | "ollama"
/// api_key:     Anthropic/OpenAI/Gemini için zorunlu, Ollama için yok sayılır
/// base_url:    sadece Ollama için (örn. "http://127.0.0.1:11434")
/// model:       opsiyonel — verilmezse provider'ın varsayılan modeli kullanılır
///
/// İlk kaydedilen provider otomatik aktif olur. Daha önce aynı
/// provider_id ile kayıt yapılmışsa (örn. key güncellendi) üzerine yazılır.
pub fn configure_ai_provider(
    provider_id: String,
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<(), String> {
    let rt = require_runtime()?;

    let provider =
        crate::ai::routing::router::build_provider(&provider_id, api_key, base_url, model)
            .map_err(|e| e.to_string())?;

    rt.ai_router.register(provider);

    info!(provider_id = %provider_id, "AI provider yapılandırıldı");
    Ok(())
}

/// Kayıtlı bir provider'ı kaldır (kullanıcı key'i sildiğinde / bağlantıyı
/// kapattığında). Kaldırılan provider aktifse aktif seçim temizlenir.
pub fn remove_ai_provider(provider_id: String) -> Result<(), String> {
    let rt = require_runtime()?;

    rt.ai_router.unregister(&provider_id);
    Ok(())
}

/// Aktif (kullanılacak) provider'ı kullanıcı seçimine göre değiştir.
/// Birden fazla provider kayıtlıysa Ayarlar ekranındaki seçim burada
/// uygulanır.
pub fn set_active_ai_provider(provider_id: String) -> Result<(), String> {
    let rt = require_runtime()?;

    rt.ai_router
        .set_active(&provider_id)
        .map_err(|e| e.to_string())
}

/// Şu an aktif olan provider_id (varsa).
pub fn get_active_ai_provider() -> Option<String> {
    get_runtime().and_then(|rt| rt.ai_router.active_id())
}

/// Kayıtlı (yapılandırılmış) tüm provider id'leri.
/// Ayarlar ekranında "hangi modeller aktif" göstermek için.
pub fn list_ai_providers() -> Vec<String> {
    get_runtime()
        .map(|rt| rt.ai_router.registered_ids())
        .unwrap_or_default()
}

/// Verilen Ollama sunucusunda yüklü (pull edilmiş) modelleri listele.
/// Ayarlar ekranındaki Ollama model dropdown'ını doldurmak için —
/// runtime başlatılmış olmasına gerek yok, doğrudan HTTP çağrısı.
pub async fn list_ollama_models(base_url: String) -> Result<Vec<String>, String> {
    crate::ai::providers::ollama::OllamaProvider::list_models(&base_url)
        .await
        .map_err(|e| e.to_string())
}

/// Ayarlar ekranındaki "Bağlantıyı Test Et" butonu için: kayıtlı bir
/// provider'a küçük bir inference isteği gönderir, kısa bir çıktı
/// parçası döner (başarılıysa key/host geçerli demektir).
pub async fn test_ai_provider(provider_id: String) -> Result<String, String> {
    let rt = require_runtime()?;

    let provider = rt
        .ai_router
        .provider(&provider_id)
        .ok_or_else(|| "Provider henüz yapılandırılmadı".to_string())?;

    let request =
        crate::ai::inference::request::InferenceRequest::new("Tek kelimeyle selam ver.", 16);

    let response = provider.infer(request).await.map_err(|e| e.to_string())?;

    Ok(response.output)
}

/// Genel amaçlı, tek seferlik AI sohbet isteği — AKTİF provider üzerinden
/// çalışır. Flutter'daki AI Chat ekranı bunu kullanır: hangi provider'ın
/// yanıt vereceği kullanıcının Ayarlar'da seçtiği aktif modele bağlıdır
/// (Anthropic/OpenAI/Gemini/Ollama — kullanıcı hangisini aktif ettiyse).
pub async fn ai_chat(
    prompt: String,
    system_prompt: Option<String>,
    max_tokens: usize,
) -> Result<String, String> {
    let rt = require_runtime()?;

    let mut request = crate::ai::inference::request::InferenceRequest::new(prompt, max_tokens);
    if let Some(system) = system_prompt {
        request = request.with_system(system);
    }

    rt.ai_router
        .infer_active(request)
        .await
        .map(|r| r.output)
        .map_err(|e| e.to_string())
}
