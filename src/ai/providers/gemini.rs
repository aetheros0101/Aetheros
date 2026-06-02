// ============================================================
// src/ai/providers/gemini.rs
//
// Faz 6 Düzeltmesi:
//
// Orijinal: AiProvider trait implement ediyordu.
//           Bu trait mevcut değil (ai/inference/traits.rs stub).
//           ModelProvider trait'i implement etmeli.
//
// Şu an stub — Google Gemini API entegrasyonu Faz 8'de.
// ============================================================

use async_trait::async_trait;

use crate::ai::errors::AiError;
use crate::ai::inference::request::InferenceRequest;
use crate::ai::inference::response::InferenceResponse;
use crate::ai::providers::provider::ModelProvider;

pub struct GeminiProvider;

#[async_trait]
impl ModelProvider for GeminiProvider {
    fn provider_id(&self) -> &'static str {
        "gemini"
    }

    fn supports_streaming(&self) -> bool {
        false
    }

    async fn infer(
        &self,
        request: InferenceRequest,
    ) -> Result<InferenceResponse, AiError> {
        // TODO Faz 8: Google Gemini API entegrasyonu
        Ok(InferenceResponse {
            output: request.prompt,
            tokens_used: 0,
        })
    }
}
