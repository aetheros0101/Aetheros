use std::sync::Arc;

use dashmap::DashMap;

use crate::plugins::traits::Plugin;

pub struct PluginRegistry {
    plugins:
        DashMap<
            String,
            Arc<dyn Plugin>,
        >,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins:
                DashMap::new(),
        }
    }

    pub fn register(
        &self,
        plugin:
            Arc<dyn Plugin>,
    ) {
        self.plugins.insert(
            plugin.name().into(),
            plugin,
        );
    }

    pub fn get(
        &self,
        name: &str,
    ) -> Option<
        Arc<dyn Plugin>,
    > {
        self.plugins
            .get(name)
            .map(|plugin| {
                Arc::clone(
                    &*plugin,
                )
            })
    }
}
