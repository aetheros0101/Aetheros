// ============================================================
// src/api/mod.rs
// ============================================================

pub mod dashboard;
pub mod models;
pub mod rest;
pub mod routes;
pub mod websocket;
pub mod middleware;

use std::net::SocketAddr;
use std::sync::Arc;

use dashmap::DashMap;
use tracing::info;

use crate::ai::routing::router::{build_provider, ProviderRouter};
use crate::api::rest::router::{
    AgentRegistry,
    WorkflowRegistry,
    build_router,
    AppState,
};
use crate::scripting::registry::ScriptRegistry;
use crate::remote::cluster::ClusterState;
use crate::events::bus::EventBus;
use crate::metrics::runtime::RuntimeMetrics;
use crate::persistence::engine::PersistenceEngine;
use crate::runtime::api::RuntimeHandle;
use crate::security::capability_engine::CapabilityEngine;
use crate::wasm::module_store::ModuleStore;

pub struct ApiServer {
    state: AppState,
    addr:  SocketAddr,
}

impl ApiServer {
    pub fn new(
        runtime:      RuntimeHandle,
        events:       EventBus,
        persistence:  Arc<PersistenceEngine>,
        metrics:      Arc<RuntimeMetrics>,
        module_store: Arc<ModuleStore>,
        addr:         SocketAddr,
    ) -> Self {
        let agent_registry:    AgentRegistry    = Arc::new(DashMap::new());
        let workflow_registry: WorkflowRegistry = Arc::new(DashMap::new());
        let script_registry                     = Arc::new(ScriptRegistry::new());
        let cluster                             = Arc::new(ClusterState::new());
        let ai_router                           = Arc::new(Self::ai_router_from_env());
        let capability_engine                   = Arc::new(CapabilityEngine::new());

        Self {
            state: AppState {
                runtime,
                events,
                persistence,
                metrics,
                module_store,
                agent_registry,
                workflow_registry,
                script_registry,
                cluster,
                ai_router,
                capability_engine,
            },
            addr,
        }
    }

    /// Sunucu/server dağıtımında AI provider'lar ortam değişkenlerinden
    /// okunur (mobil tarafta bunun yerine Flutter Ayarlar ekranından
    /// configure_ai_provider bridge fonksiyonu kullanılır).
    ///
    ///   ANTHROPIC_API_KEY [+ ANTHROPIC_MODEL]
    ///   OPENAI_API_KEY    [+ OPENAI_MODEL]
    ///   GEMINI_API_KEY    [+ GEMINI_MODEL]
    ///   OLLAMA_HOST       [+ OLLAMA_MODEL]   (key gerekmez)
    ///
    /// Hiçbiri set değilse router boş kalır — agent/workflow AI
    /// adımları sessizce fallback'e düşer, sunucu yine de açılır.
    fn ai_router_from_env() -> ProviderRouter {
        let router = ProviderRouter::new();

        let registrations: [(&str, &str, &str); 4] = [
            ("anthropic", "ANTHROPIC_API_KEY", "ANTHROPIC_MODEL"),
            ("openai",    "OPENAI_API_KEY",    "OPENAI_MODEL"),
            ("gemini",    "GEMINI_API_KEY",    "GEMINI_MODEL"),
            ("ollama",    "OLLAMA_HOST",       "OLLAMA_MODEL"),
        ];

        for (provider_id, primary_var, model_var) in registrations {
            let Ok(primary_value) = std::env::var(primary_var) else {
                continue;
            };
            let model = std::env::var(model_var).ok();

            // Ollama'da "primary_var" key değil host'tur; diğerlerinde key'dir.
            let (api_key, base_url) = if provider_id == "ollama" {
                (None, Some(primary_value))
            } else {
                (Some(primary_value), None)
            };

            match build_provider(provider_id, api_key, base_url, model) {
                Ok(provider) => {
                    info!(provider_id, "AI provider env'den yapılandırıldı");
                    router.register(provider);
                }
                Err(e) => {
                    tracing::warn!(provider_id, error = %e, "AI provider yapılandırılamadı");
                }
            }
        }

        router
    }

    pub async fn serve(
        self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let app = build_router().with_state(self.state);

        info!(addr = %self.addr, "API server listening");
        info!("Dashboard: http://{}/dashboard", self.addr);

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
