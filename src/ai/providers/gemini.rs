// ============================================================
// src/ai/providers/gemini.rs
//
// Google Gemini API provider (ücretsiz tier)
// Model: gemini-flash-latest (otomatik güncel alias — Haziran 2026)
// Endpoint: POST /v1beta/models/{model}:generateContent?key={key}
// ============================================================

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::ai::errors::AiError;
use crate::ai::inference::request::InferenceRequest;
use crate::ai::inference::response::InferenceResponse;
use crate::ai::providers::provider::ModelProvider;

// ── Request şeması ────────────────────────────────────────

#[derive(Debug, Serialize)]
struct GeminiRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction<'a>>,
    contents: Vec<GeminiContent<'a>>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Serialize)]
struct GeminiSystemInstruction<'a> {
    parts: Vec<GeminiPart<'a>>,
}

#[derive(Debug, Serialize)]
struct GeminiContent<'a> {
    role: &'a str,
    parts: Vec<GeminiPart<'a>>,
}

#[derive(Debug, Serialize)]
struct GeminiPart<'a> {
    text: &'a str,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    temperature: f32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: usize,
    #[serde(rename = "topP")]
    top_p: f32,
}

// ── Response şeması ───────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsage>,
    error: Option<GeminiError>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiResponseContent>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponsePart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiUsage {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<usize>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    code: u16,
    message: String,
}

// ── Provider ──────────────────────────────────────────────

pub struct GeminiProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl GeminiProvider {
    /// Varsayılan model: gemini-flash-latest (ücretsiz tier).
    ///
    /// NOT: gemini-2.0-flash 1 Haziran 2026'da kapatıldı (sabit sürüm
    /// isimleri zamanla kullanımdan kaldırılıyor). "latest" alias'ları
    /// Google tarafından otomatik güncel modele yönlendirilir — şu an
    /// gemini-flash-latest → Gemini 3.5 Flash (GA, Mayıs 2026).
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "gemini-flash-latest".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    fn endpoint(&self) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        )
    }
}

#[async_trait]
impl ModelProvider for GeminiProvider {
    fn provider_id(&self) -> &'static str {
        "gemini"
    }

    fn supports_streaming(&self) -> bool {
        false
    }

    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse, AiError> {
        debug!(model = %self.model, "Gemini inference request");

        let user_part = GeminiPart {
            text: &request.prompt,
        };

        let _system_instruction =
            request
                .system_prompt
                .as_deref()
                .map(|s| GeminiSystemInstruction {
                    parts: vec![GeminiPart { text: s }],
                });

        // system_prompt str'ini &str olarak tutmak için
        let sp_owned = request.system_prompt.clone().unwrap_or_default();
        let system_instruction = if !sp_owned.is_empty() {
            Some(GeminiSystemInstruction {
                parts: vec![GeminiPart { text: &sp_owned }],
            })
        } else {
            None
        };

        let body = GeminiRequest {
            system_instruction,
            contents: vec![GeminiContent {
                role: "user",
                parts: vec![user_part],
            }],
            generation_config: GeminiGenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
                top_p: 0.9,
            },
        };

        let response = self
            .client
            .post(self.endpoint())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    warn!("Gemini'ye bağlanılamadı (ağ sorunu)");
                    AiError::ProviderFailure {
                        message: "Bağlantı kurulamadı — internet bağlantını \
                                  kontrol et ve tekrar dene."
                            .to_string(),
                    }
                } else {
                    AiError::ProviderFailure {
                        message: format!("HTTP hatası: {e}"),
                    }
                }
            })?;

        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            warn!("Gemini rate limit aşıldı");
            return Err(AiError::RateLimited);
        }

        let gemini_resp: GeminiResponse =
            response
                .json()
                .await
                .map_err(|e| AiError::ProviderFailure {
                    message: format!("JSON parse hatası: {e}"),
                })?;

        // API hata yanıtı
        if let Some(err) = gemini_resp.error {
            warn!(code = err.code, msg = %err.message, "Gemini API hatası");
            return Err(if err.code == 429 {
                AiError::RateLimited
            } else {
                AiError::ProviderFailure {
                    message: format!("Gemini {}: {}", err.code, err.message),
                }
            });
        }

        let output = gemini_resp
            .candidates
            .as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.content.as_ref())
            .and_then(|c| c.parts.first())
            .and_then(|p| p.text.clone())
            .unwrap_or_default();

        let tokens_used = gemini_resp
            .usage_metadata
            .map(|u| u.prompt_token_count.unwrap_or(0) + u.candidates_token_count.unwrap_or(0))
            .unwrap_or(0);

        debug!(tokens_used, "Gemini inference tamamlandı");

        Ok(InferenceResponse {
            output,
            tokens_used,
        })
    }
}
