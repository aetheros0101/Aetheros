// ============================================================
// src/ai/providers/openai.rs
//
// OpenAI Chat Completions API provider.
//
// ModelProvider trait implementasyonu:
//   - gpt-5.5 varsayılan model (Haziran 2026 itibarıyla genel
//     kullanıma açık flagship model; gpt-5.6 ailesi henüz sınırlı
//     preview'da, herkesin API key'i erişemeyebilir)
//   - Chat Completions API (/v1/chat/completions)
//   - max_completion_tokens kullanılır — max_tokens GPT-5 sınıfı
//     reasoning modellerinde artık desteklenmiyor (deprecated).
//   - Streaming bu sürümde desteklenmiyor (supports_streaming: false)
//
// Kimlik doğrulama:
//   Kullanıcının uygulama içinde girdiği API key (from_key).
//   Sunucu/dev tarafı için OPENAI_API_KEY environment variable
//   fallback olarak da okunabilir (new()).
//
// Hata Sınıflandırması:
//   429 Too Many Requests → AiError::RateLimited
//   5xx Server Error      → AiError::ProviderFailure
//   diğer                  → AiError::ProviderFailure { message }
// ============================================================

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::ai::errors::AiError;
use crate::ai::inference::request::InferenceRequest;
use crate::ai::inference::response::InferenceResponse;
use crate::ai::providers::provider::ModelProvider;

// ── OpenAI API request/response şemaları ──────────────────

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    max_completion_tokens: usize,
    temperature: f64,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    #[serde(default)]
    prompt_tokens: usize,
    #[serde(default)]
    completion_tokens: usize,
}

// ── Provider ──────────────────────────────────────────────

pub struct OpenAiProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    /// OPENAI_API_KEY environment variable'dan oluştur (sunucu/dev).
    pub fn new() -> Result<Self, AiError> {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            warn!("OPENAI_API_KEY not set");
            AiError::ProviderUnavailable
        })?;

        Ok(Self::from_key(api_key))
    }

    /// Kullanıcının uygulama içinde girdiği API key ile oluştur.
    ///
    /// Varsayılan model: gpt-5.5 (genel kullanıma açık flagship model).
    /// Daha ucuz/hızlı bir seçenek isteniyorsa with_model("gpt-5.4-mini").
    pub fn from_key(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "gpt-5.5".to_string(),
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
impl ModelProvider for OpenAiProvider {
    fn provider_id(&self) -> &'static str {
        "openai"
    }

    fn supports_streaming(&self) -> bool {
        false
    }

    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse, AiError> {
        debug!(
            model = %self.model,
            max_tokens = request.max_tokens,
            "OpenAI inference request"
        );

        let mut messages = Vec::with_capacity(2);
        if let Some(system) = request.system_prompt.clone() {
            messages.push(OpenAiMessage {
                role: "system".to_string(),
                content: system,
            });
        }
        messages.push(OpenAiMessage {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });

        let body = OpenAiRequest {
            model: self.model.clone(),
            messages,
            max_completion_tokens: request.max_tokens,
            temperature: request.temperature as f64,
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    warn!("OpenAI'a bağlanılamadı (ağ sorunu)");
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
            warn!("OpenAI rate limit exceeded");
            return Err(AiError::RateLimited);
        }

        if status.is_server_error() {
            return Err(AiError::ProviderFailure {
                message: format!("OpenAI server error: {}", status),
            });
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::ProviderFailure {
                message: format!("OpenAI error {}: {}", status, body),
            });
        }

        let openai_resp: OpenAiResponse =
            response
                .json()
                .await
                .map_err(|e| AiError::ProviderFailure {
                    message: e.to_string(),
                })?;

        let output = openai_resp
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let tokens_used = openai_resp
            .usage
            .map(|u| u.prompt_tokens + u.completion_tokens)
            .unwrap_or(0);

        debug!(tokens_used, "OpenAI inference complete");

        Ok(InferenceResponse {
            output,
            tokens_used,
        })
    }
}
