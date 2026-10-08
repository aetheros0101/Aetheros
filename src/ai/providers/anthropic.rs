// ============================================================
// src/ai/providers/anthropic.rs  (YENİ)
//
// Anthropic Claude API provider.
//
// ModelProvider trait implementasyonu:
//   - claude-sonnet-4-6 varsayılan model
//   - Messages API (/v1/messages)
//   - Streaming destekleniyor (supports_streaming: true)
//   - Token budget takibi (AgentExecutionBudget ile uyumlu)
//
// Kimlik doğrulama:
//   ANTHROPIC_API_KEY environment variable'dan okunur.
//   Runtime başlangıcında eksikse ProviderUnavailable döner.
//
// Hata Sınıflandırması:
//   429 Too Many Requests → AiError::RateLimited
//   5xx Server Error      → AiError::ProviderFailure
//   diğer                 → AiError::ProviderFailure { message }
// ============================================================

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::ai::errors::AiError;
use crate::ai::inference::request::InferenceRequest;
use crate::ai::inference::response::InferenceResponse;
use crate::ai::providers::provider::ModelProvider;

// ── Anthropic API request/response şemaları ───────────────

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: usize,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: usize,
    output_tokens: usize,
}

// ── Provider ──────────────────────────────────────────────

pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    /// Varsayılan model: claude-sonnet-4-6 (Haziran 2026 itibarıyla güncel).
    pub fn new() -> Result<Self, AiError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
            warn!("ANTHROPIC_API_KEY not set");
            AiError::ProviderUnavailable
        })?;

        Ok(Self::from_key(api_key))
    }

    /// Kullanıcının uygulama içinde girdiği API key ile oluştur.
    /// Mobil tarafta env var yok — Flutter Settings ekranından
    /// gelen key doğrudan buraya geçirilir (bkz. configure_ai_provider).
    pub fn from_key(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "claude-sonnet-4-6".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Farklı bir model ile oluştur.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait]
impl ModelProvider for AnthropicProvider {
    fn provider_id(&self) -> &'static str {
        "anthropic"
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse, AiError> {
        debug!(
            model = %self.model,
            max_tokens = request.max_tokens,
            "Anthropic inference request"
        );

        let body = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: request.max_tokens,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: request.prompt.clone(),
            }],
            system: request.system_prompt.clone(),
            temperature: Some(request.temperature as f64),
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    warn!("Anthropic'e bağlanılamadı (ağ sorunu)");
                    AiError::ProviderFailure {
                        message: "Bağlantı kurulamadı — internet bağlantını \
                                  kontrol et ve tekrar dene."
                            .to_string(),
                    }
                } else {
                    AiError::ProviderFailure {
                        message: e.to_string(),
                    }
                }
            })?;

        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            warn!("Anthropic rate limit exceeded");
            return Err(AiError::RateLimited);
        }

        if status.is_server_error() {
            return Err(AiError::ProviderFailure {
                message: format!("Anthropic server error: {}", status),
            });
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::ProviderFailure {
                message: format!("Anthropic error {}: {}", status, body),
            });
        }

        let anthropic_resp: AnthropicResponse =
            response
                .json()
                .await
                .map_err(|e| AiError::ProviderFailure {
                    message: e.to_string(),
                })?;

        // İlk text content bloğunu al
        let output = anthropic_resp
            .content
            .iter()
            .find(|c| c.kind == "text")
            .and_then(|c| c.text.clone())
            .unwrap_or_default();

        let tokens_used = anthropic_resp.usage.input_tokens + anthropic_resp.usage.output_tokens;

        debug!(tokens_used, "Anthropic inference complete");

        Ok(InferenceResponse {
            output,
            tokens_used,
        })
    }
}
