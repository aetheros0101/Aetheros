//! LLM provider portu — Intent crate belirli bir vendor'a bağlı değildir.
//!
//! Host (AetherOS AI router) bu trait'i implement eder.

use crate::errors::{IntentError, Result};
use serde::{Deserialize, Serialize};

/// Structured completion isteği.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredRequest {
    pub system: String,
    pub user: String,
    /// JSON schema ipucu (provider'a göre uygulanır).
    #[serde(default)]
    pub schema_name: Option<String>,
    #[serde(default)]
    pub temperature: Option<f32>,
}

/// Structured completion yanıtı.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredResponse {
    /// Ham model metni (genelde JSON).
    pub content: String,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub tokens_used: Option<u32>,
}

/// Provider-agnostic structured output portu.
pub trait StructuredLlm: Send + Sync {
    fn complete_structured(&self, request: &StructuredRequest) -> Result<StructuredResponse>;
}

/// Test / offline: sabit JSON döner.
#[derive(Debug, Clone)]
pub struct StaticJsonLlm {
    pub content: String,
}

impl StructuredLlm for StaticJsonLlm {
    fn complete_structured(&self, _request: &StructuredRequest) -> Result<StructuredResponse> {
        Ok(StructuredResponse {
            content: self.content.clone(),
            model_id: Some("static".into()),
            tokens_used: Some(0),
        })
    }
}

/// Bilinçli olarak hata veren provider (test).
#[derive(Debug, Default)]
pub struct FailingLlm;

impl StructuredLlm for FailingLlm {
    fn complete_structured(&self, _: &StructuredRequest) -> Result<StructuredResponse> {
        Err(IntentError::Extraction("llm unavailable".into()))
    }
}

pub fn parse_json<T: for<'de> Deserialize<'de>>(content: &str) -> Result<T> {
    // Allow fenced ```json blocks
    let trimmed = content.trim();
    let json = if let Some(start) = trimmed.find('{') {
        let end = trimmed.rfind('}').map(|i| i + 1).unwrap_or(trimmed.len());
        &trimmed[start..end]
    } else {
        trimmed
    };
    serde_json::from_str(json).map_err(|e| IntentError::Extraction(format!("json parse: {e}")))
}
