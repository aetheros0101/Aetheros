// ============================================================
// src/ai/inference/request.rs
//
// Güncelleme:
//   system_prompt alanı eklendi.
//   Anthropic Messages API'sinde system prompt
//   user message'dan ayrı gönderilir.
//   Option<String> — verilmezse None, API'ye gönderilmez.
// ============================================================

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub prompt: String,

    /// Anthropic: system field olarak gönderilir.
    /// OpenAI/Gemini: system role mesajı olarak eklenir.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    pub temperature: f32,

    pub max_tokens: usize,
}

impl InferenceRequest {
    pub fn new(prompt: impl Into<String>, max_tokens: usize) -> Self {
        Self {
            prompt: prompt.into(),
            system_prompt: None,
            temperature: 0.7,
            max_tokens,
        }
    }

    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system_prompt = Some(system.into());
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }
}
