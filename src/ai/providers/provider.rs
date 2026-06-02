use async_trait::async_trait;

use crate::ai::errors::AiError;

use crate::ai::inference::request::InferenceRequest;

use crate::ai::inference::response::InferenceResponse;

#[async_trait]
pub trait ModelProvider:
    Send + Sync
{
    fn provider_id(
        &self,
    ) -> &'static str;

    fn supports_streaming(
        &self,
    ) -> bool;

    async fn infer(
        &self,
        request:
            InferenceRequest,
    ) -> Result<
        InferenceResponse,
        AiError,
    >;
}
