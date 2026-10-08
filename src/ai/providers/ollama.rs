// ============================================================
// src/ai/providers/ollama.rs
//
// Ollama (local) provider — kullanıcının kendi cihazında veya
// yerel ağındaki bir Ollama sunucusuna bağlanır.
//
// ModelProvider trait implementasyonu:
//   - Varsayılan host: http://127.0.0.1:11434
//     (Termux'ta `ollama serve` ile aynı cihazda da çalışabilir)
//   - Endpoint: POST /api/generate (tek turlu, stream:false)
//   - API key YOK — Ollama lokal sunucu, kimlik doğrulama gerekmez.
//     Kullanıcı sadece host (ve istediği model adını) girer.
//
// Model yönetimi kullanıcıya bırakılmıştır: model cihazda/sunucuda
// önceden `ollama pull <model>` ile indirilmiş olmalı.
// list_models() ile sunucudaki yüklü modeller sorgulanabilir
// (Ayarlar ekranında dropdown doldurmak için kullanılır).
//
// Hata Sınıflandırması:
//   Bağlantı hatası (sunucu kapalı) → AiError::ProviderUnavailable
//   404 (model bulunamadı)          → AiError::ProviderFailure
//   diğer                            → AiError::ProviderFailure { message }
// ============================================================

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::ai::errors::AiError;
use crate::ai::inference::request::InferenceRequest;
use crate::ai::inference::response::InferenceResponse;
use crate::ai::providers::provider::ModelProvider;

// ── Ollama API request/response şemaları ──────────────────

#[derive(Debug, Serialize)]
struct OllamaGenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    #[serde(default)]
    prompt_eval_count: usize,
    #[serde(default)]
    eval_count: usize,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaModelEntry>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelEntry {
    name: String,
}

// ── Provider ──────────────────────────────────────────────

pub struct OllamaProvider {
    /// Örn: "http://127.0.0.1:11434" (sondaki "/" olmadan)
    host: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    /// Varsayılan host (127.0.0.1:11434) ve verilen modelle oluştur.
    pub fn new(model: impl Into<String>) -> Self {
        Self::with_host("http://127.0.0.1:11434", model)
    }

    /// Özel host (örn. yerel ağda başka bir cihaz) ile oluştur.
    pub fn with_host(host: impl Into<String>, model: impl Into<String>) -> Self {
        let host: String = host.into();
        Self {
            host: host.trim_end_matches('/').to_string(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Sunucudaki yüklü (pull edilmiş) modelleri listele.
    /// Ayarlar ekranında "Model seç" dropdown'ını doldurmak için.
    pub async fn list_models(host: &str) -> Result<Vec<String>, AiError> {
        let host = host.trim_end_matches('/');
        let client = reqwest::Client::new();

        let response = client
            .get(format!("{host}/api/tags"))
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    AiError::ProviderUnavailable
                } else {
                    AiError::ProviderFailure {
                        message: e.to_string(),
                    }
                }
            })?;

        if !response.status().is_success() {
            return Err(AiError::ProviderFailure {
                message: format!("Ollama /api/tags hatası: {}", response.status()),
            });
        }

        let tags: OllamaTagsResponse =
            response
                .json()
                .await
                .map_err(|e| AiError::ProviderFailure {
                    message: e.to_string(),
                })?;

        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }
}

#[async_trait]
impl ModelProvider for OllamaProvider {
    fn provider_id(&self) -> &'static str {
        "ollama"
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse, AiError> {
        debug!(
            host = %self.host,
            model = %self.model,
            "Ollama inference request"
        );

        let body = OllamaGenerateRequest {
            model: &self.model,
            prompt: &request.prompt,
            system: request.system_prompt.as_deref(),
            stream: false,
            options: OllamaOptions {
                temperature: request.temperature,
            },
        };

        let response = self
            .client
            .post(format!("{}/api/generate", self.host))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    warn!(host = %self.host, "Ollama sunucusuna bağlanılamadı");
                    AiError::ProviderUnavailable
                } else {
                    AiError::ProviderFailure {
                        message: e.to_string(),
                    }
                }
            })?;

        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AiError::ProviderFailure {
                message: format!(
                    "Model '{}' sunucuda yüklü değil. Önce `ollama pull {}` çalıştır.",
                    self.model, self.model
                ),
            });
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::ProviderFailure {
                message: format!("Ollama hatası {}: {}", status, body),
            });
        }

        let ollama_resp: OllamaGenerateResponse =
            response
                .json()
                .await
                .map_err(|e| AiError::ProviderFailure {
                    message: e.to_string(),
                })?;

        let tokens_used = ollama_resp.prompt_eval_count + ollama_resp.eval_count;

        debug!(tokens_used, "Ollama inference complete");

        Ok(InferenceResponse {
            output: ollama_resp.response,
            tokens_used,
        })
    }
}
