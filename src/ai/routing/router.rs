use std::sync::Arc;

use dashmap::DashMap;

use crate::ai::providers::provider::ModelProvider;

pub struct ProviderRouter {
    providers:
        DashMap<
            String,
            Arc<
                dyn ModelProvider,
            >,
        >,
}

impl ProviderRouter {
    pub fn new() -> Self {
        Self {
            providers:
                DashMap::new(),
        }
    }

    pub fn register(
        &self,
        provider:
            Arc<
                dyn ModelProvider,
            >,
    ) {
        self.providers.insert(
            provider
                .provider_id()
                .into(),
            provider,
        );
    }

    pub fn provider(
        &self,
        provider_id:
            &str,
    ) -> Option<
        Arc<
            dyn ModelProvider,
        >,
    > {
        self.providers
            .get(provider_id)
            .map(|entry| {
                Arc::clone(
                    entry.value(),
                )
            })
    }
}
