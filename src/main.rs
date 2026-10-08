// ============================================================
// src/main.rs  (v2)
//
// Sprint 8: RuntimeMetrics collector başlatıldı.
//           ApiServer'a metrics geçildi.
// ============================================================

use std::net::SocketAddr;
use std::sync::Arc;

use tracing::{error, info};

use aetheros::api::ApiServer;
use aetheros::logging::TelemetrySystem;
use aetheros::metrics::runtime::RuntimeMetrics;
use aetheros::runtime::bootstrap::RuntimeBootstrap;
use aetheros::runtime::config::RuntimeConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── 1. Telemetry ──────────────────────────────────────
    let json_logs = std::env::var("AETHEROS_JSON_LOGS")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);

    if json_logs {
        TelemetrySystem::init_json();
    } else {
        TelemetrySystem::init();
    }

    info!(version = env!("CARGO_PKG_VERSION"), "AetherOS starting");

    // ── 2. Config ─────────────────────────────────────────
    let config = RuntimeConfig {
        worker_count: env_usize("AETHEROS_WORKERS", 4),
        task_channel_capacity: env_usize("AETHEROS_TASK_CHANNEL", 1024),
        event_channel_capacity: env_usize("AETHEROS_EVENT_CHANNEL", 2048),
        max_concurrent_tasks: env_usize("AETHEROS_MAX_CONCURRENT", 256),
        persistence_path: std::env::var("AETHEROS_DB_PATH")
            .unwrap_or_else(|_| "./aetheros.db".into()),
        shutdown_timeout: std::time::Duration::from_secs(
            env_usize("AETHEROS_SHUTDOWN_TIMEOUT", 30) as u64,
        ),
    };

    info!(
        workers = config.worker_count,
        max_concurrent = config.max_concurrent_tasks,
        db_path = %config.persistence_path,
        "Runtime config loaded"
    );

    // ── 3. Bootstrap ──────────────────────────────────────
    let bootstrap = RuntimeBootstrap::build(config).map_err(|e| {
        error!(error = ?e, "Failed to build runtime");
        Box::new(e) as Box<dyn std::error::Error>
    })?;

    let handle = bootstrap.runtime_handle();
    let runtime = bootstrap.runtime();
    let events = runtime.events();
    let persistence = runtime.persistence();
    let module_store = runtime.module_store();

    // ── 4. Metrics Collector ──────────────────────────────
    let metrics = Arc::new(RuntimeMetrics::new());
    metrics.clone().start_collecting(events.clone());

    // ── 5. Runtime — arka planda ─────────────────────────
    let runtime_join = tokio::spawn(async move {
        if let Err(e) = runtime.start().await {
            error!(error = ?e, "Runtime error");
        }
    });

    // ── 6. API Server ─────────────────────────────────────
    let addr: SocketAddr = std::env::var("AETHEROS_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".into())
        .parse()?;

    let server = ApiServer::new(
        handle,
        events,
        Arc::clone(&persistence),
        metrics,
        module_store,
        addr,
    );

    // ── 7. Graceful Shutdown ──────────────────────────────
    tokio::select! {
        result = server.serve() => {
            if let Err(e) = result {
                error!(error = ?e, "API server error");
            }
        }
        _ = shutdown_signal() => {
            info!("Shutdown signal received");
        }
    }

    let _ = tokio::time::timeout(std::time::Duration::from_secs(10), runtime_join).await;

    info!("AetherOS stopped");
    Ok(())
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        tokio::select! {
            _ = sigterm.recv() => info!("SIGTERM received"),
            _ = sigint.recv()  => info!("SIGINT received"),
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.ok();
    }
}
