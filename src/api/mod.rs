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
            },
            addr,
        }
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
