pub mod policy;
pub mod router;

use std::sync::Arc;

use crate::ai::errors::AiError;

use crate::ai::inference::request::InferenceRequest;

use crate::ai::inference::response::InferenceResponse;

use crate::ai::inference::traits::AiProvider;

pub struct ProviderRouter {
    providers:
        Vec<
            Arc<
                dyn AiProvider,
            >,
        >,
}

impl ProviderRouter {
    pub fn new() -> Self {
        Self {
            providers:
                Vec::new(),
        }
    }

    pub fn register(
        &mut self,
        provider:
            Arc<
                dyn AiProvider,
            >,
    ) {
        self.providers
            .push(provider);
    }

    pub async fn infer(
        &self,
        request:
            InferenceRequest,
    ) -> Result<
        InferenceResponse,
        AiError,
    > {
        let provider =
            self.providers
                .first()
                .ok_or(
                    AiError::ProviderUnavailable,
                )?;

        provider
            .infer(request)
            .await
    }
}
