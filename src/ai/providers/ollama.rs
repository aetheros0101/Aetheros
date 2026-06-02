use async_trait::async_trait;

use crate::ai::errors::AiError;

use crate::ai::inference::request::InferenceRequest;

use crate::ai::inference::response::InferenceResponse;

use crate::ai::providers::provider::ModelProvider;

pub struct OllamaProvider;

#[async_trait]
impl ModelProvider
    for OllamaProvider
{
    fn provider_id(
        &self,
    ) -> &'static str {
        "ollama"
    }

    fn supports_streaming(
        &self,
    ) -> bool {
        true
    }

    async fn infer(
        &self,
        request:
            InferenceRequest,
    ) -> Result<
        InferenceResponse,
        AiError,
    > {
        Ok(
            InferenceResponse {
                output:
                    request.prompt,

                tokens_used:
                    0,
            },
        )
    }
}
