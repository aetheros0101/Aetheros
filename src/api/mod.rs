// ============================================================
// src/api/mod.rs  (v2)
//
// Sprint 8: dashboard + metrics AppState'e eklendi.
// ============================================================

pub mod dashboard;
pub mod models;
pub mod rest;
pub mod routes;
pub mod websocket;
pub mod middleware;

use std::net::SocketAddr;
use std::sync::Arc;

use tracing::info;

use crate::api::rest::router::{
    build_router,
    AppState,
};
use crate::events::bus::EventBus;
use crate::metrics::runtime::RuntimeMetrics;
use crate::persistence::engine::PersistenceEngine;
use crate::runtime::api::RuntimeHandle;

pub struct ApiServer {
    state: AppState,
    addr: SocketAddr,
}

impl ApiServer {
    pub fn new(
        runtime: RuntimeHandle,
        events: EventBus,
        persistence: Arc<PersistenceEngine>,
        metrics: Arc<RuntimeMetrics>,
        addr: SocketAddr,
    ) -> Self {
        Self {
            state: AppState {
                runtime,
                events,
                persistence,
                metrics,
            },
            addr,
        }
    }

    pub async fn serve(
        self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let app = build_router()
            .with_state(self.state);

        info!(addr = %self.addr, "API server listening");
        info!("Dashboard: http://{}/dashboard", self.addr);

        let listener =
            tokio::net::TcpListener::bind(self.addr)
                .await?;

        axum::serve(listener, app).await?;

        Ok(())
    }
}
