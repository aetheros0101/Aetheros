// ============================================================
// src/api/rest/router.rs  (v2)
//
// Sprint 8 Eklemeleri:
//   GET /dashboard → HTML realtime dashboard
//   GET /metrics   → JSON metrik snapshot
// ============================================================

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json,
    Router,
};
use chrono::Utc;
use uuid::Uuid;

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::executor::AgentExecutor;
use crate::api::dashboard::dashboard_handler;
use crate::events::bus::EventBus;
use crate::metrics::runtime::RuntimeMetrics;
use crate::persistence::engine::PersistenceEngine;
use crate::runtime::api::RuntimeHandle;
use crate::task::priority::TaskPriority;
use crate::task::retry::RetryPolicy;
use crate::task::task::{
    TaskDefinition,
    TaskMetadata,
    TaskState,
};
use crate::types::ids::TaskId;

// ── AppState ──────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub runtime: RuntimeHandle,
    pub events: EventBus,
    pub persistence: Arc<PersistenceEngine>,
    /// Sprint 8: metrics dashboard için
    pub metrics: Arc<RuntimeMetrics>,
}

// ── Router ────────────────────────────────────────────────

pub fn build_router() -> Router<AppState> {
    Router::new()
        // Dashboard
        .route("/dashboard", get(dashboard_handler))
        .route("/metrics",   get(metrics_handler))
        // Tasks
        .route("/health",              get(health_handler))
        .route("/tasks",               post(submit_task_handler))
        .route("/tasks/{id}",        get(task_state_handler))
        .route("/tasks/{id}/cancel", post(cancel_task_handler))
        // Agents & Workflows
        .route("/agents",              post(start_agent_handler))
        .route("/workflows",           post(submit_workflow_handler))
        .route("/ws", get(crate::api::websocket::ws_upgrade_handler))
}

// ── Handlers ──────────────────────────────────────────────

async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "healthy": true,
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": Utc::now().to_rfc3339(),
    }))
}

/// GET /metrics → RuntimeMetrics snapshot
async fn metrics_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let snap = state.metrics.snapshot();
    Json(serde_json::json!({
        "active_workers":  snap.active_workers,
        "queued_tasks":    snap.queued_tasks,
        "completed_tasks": snap.completed_tasks,
        "failed_tasks":    snap.failed_tasks,
        "retried_tasks":   snap.retried_tasks,
        "timestamp":       Utc::now().to_rfc3339(),
    }))
}

async fn submit_task_handler(
    State(state): State<AppState>,
    Json(req): Json<TaskSubmitRequest>,
) -> impl IntoResponse {
    let task_id = TaskId(Uuid::new_v4());

    let _wasm_module = match hex::decode(&req.wasm_module_hex) {
        Ok(bytes) => bytes,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "invalid wasm_module_hex"
                })),
            ).into_response();
        }
    };

    let priority = match req.priority.as_deref() {
        Some("critical") => TaskPriority::Critical,
        Some("high")     => TaskPriority::High,
        Some("low")      => TaskPriority::Low,
        _                => TaskPriority::Normal,
    };

    let task = TaskDefinition {
        id: task_id,
        parent: None,
        orchestration: None,
        priority,
        deadline: None,
        timeout_ms: req.timeout_ms,
        retry_policy: RetryPolicy {
            max_attempts: req.max_attempts,
            base_delay_ms: 500,
            max_delay_ms: 30_000,
            jitter: true,
        },
        metadata: TaskMetadata {
            labels: Default::default(),
        },
        wasm_module_hash: [0u8; 32],
        entrypoint: req.entrypoint,
        state: TaskState::Queued,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    match state.runtime.submit(task).await {
        Ok(()) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "task_id": task_id.0,
                "status": "queued"
            })),
        ).into_response(),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
    }
}

async fn task_state_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let task_id = TaskId(id);
    match state.persistence.load_task(&task_id) {
        Ok(Some(p)) => Json(serde_json::json!({
            "task_id": id,
            "state": format!("{:?}", p.task.state),
            "attempts": p.attempts,
            "created_at": p.created_at,
            "updated_at": p.updated_at,
        })).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "task not found" })),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
    }
}

async fn cancel_task_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let task_id = TaskId(id);
    match state.persistence.update_task_state(&task_id, TaskState::Cancelled) {
        Ok(()) => Json(serde_json::json!({
            "task_id": id, "status": "cancelled"
        })).into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "task not found" })),
        ).into_response(),
    }
}

async fn start_agent_handler(
    State(_state): State<AppState>,
    Json(req): Json<AgentStartRequest>,
) -> impl IntoResponse {
    let execution_id = Uuid::new_v4();
    let agent_id = Uuid::new_v4();

    let context = AgentContext { agent_id, execution_id, workflow_id: None };
    let budget = AgentExecutionBudget {
        max_tokens: req.max_tokens,
        max_steps: req.max_steps,
        max_runtime_seconds: 300,
    };

    tokio::spawn(async move {
        let _ = AgentExecutor::execute(context, req.objective, budget, vec![]).await;
    });

    (StatusCode::ACCEPTED, Json(serde_json::json!({
        "execution_id": execution_id,
        "status": "started"
    }))).into_response()
}

async fn submit_workflow_handler(
    State(_state): State<AppState>,
    Json(req): Json<WorkflowSubmitRequest>,
) -> impl IntoResponse {
    let workflow_id = Uuid::new_v4();
    (StatusCode::ACCEPTED, Json(serde_json::json!({
        "workflow_id": workflow_id,
        "status": "accepted",
        "name": req.name
    }))).into_response()
}

// ── Request Modelleri ─────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct TaskSubmitRequest {
    pub entrypoint: String,
    pub wasm_module_hex: String,
    pub priority: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_attempts")]
    pub max_attempts: u32,
}
fn default_timeout() -> u64 { 30_000 }
fn default_attempts() -> u32 { 3 }

#[derive(serde::Deserialize)]
pub struct AgentStartRequest {
    pub objective: String,
    #[serde(default = "default_steps")]
    pub max_steps: usize,
    #[serde(default = "default_tokens")]
    pub max_tokens: usize,
}
fn default_steps() -> usize { 10 }
fn default_tokens() -> usize { 4096 }

#[derive(serde::Deserialize)]
pub struct WorkflowSubmitRequest {
    pub name: String,
}
