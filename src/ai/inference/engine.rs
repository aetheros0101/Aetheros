use std::sync::Arc;

use crate::ai::errors::AiError;

use crate::ai::inference::request::InferenceRequest;

use crate::ai::inference::response::InferenceResponse;

use crate::ai::inference::traits::AiProvider;

pub struct InferenceEngine {
    provider:
        Arc<dyn AiProvider>,
}

impl InferenceEngine {
    pub fn new(
        provider:
            Arc<dyn AiProvider>,
    ) -> Self {
        Self { provider }
    }

    pub async fn infer(
        &self,
        request:
            InferenceRequest,
    ) -> Result<
        InferenceResponse,
        AiError,
    > {
        self.provider
            .infer(request)
            .await
    }
}
