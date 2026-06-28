// ============================================================
// src/api/rest/router.rs
//
// Adım 1: submit_task_handler → gerçek ModuleStore kaydı
// Adım 2: Agent execution registry + GET /agents/{id}
// Adım 3: WorkflowEngine bağlantısı + GET /workflows/{id}
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
use dashmap::DashMap;
use uuid::Uuid;

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::executor::AgentExecutor;
use crate::ai::routing::router::ProviderRouter;
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
use crate::wasm::module_store::ModuleStore;
use crate::workflows::compiler::{StepDsl, WorkflowDsl};
use crate::workflows::engine::WorkflowEngine;
use crate::scripting::definition::ScriptDefinition;
use crate::scripting::registry::ScriptRegistry;
use crate::remote::cluster::ClusterState;
use crate::remote::node::{NodeCapability, RemoteNode};
use crate::remote::protocol::RemoteCommand;

// ── Agent durumu ──────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentStatus {
    pub execution_id: Uuid,
    pub agent_id:     Uuid,
    pub objective:    String,
    /// "running" | "completed" | "failed"
    pub status:       String,
    pub error:        Option<String>,
    pub started_at:   chrono::DateTime<Utc>,
    pub finished_at:  Option<chrono::DateTime<Utc>>,
}

pub type AgentRegistry = Arc<DashMap<Uuid, AgentStatus>>;

// ── Workflow durumu ───────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkflowStatus {
    pub workflow_id: Uuid,
    pub name:        String,
    /// "running" | "completed" | "failed"
    pub status:      String,
    pub error:       Option<String>,
    pub started_at:  chrono::DateTime<Utc>,
    pub finished_at: Option<chrono::DateTime<Utc>>,
}

pub type WorkflowRegistry = Arc<DashMap<Uuid, WorkflowStatus>>;

// ── AppState ──────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub runtime:           RuntimeHandle,
    pub events:            EventBus,
    pub persistence:       Arc<PersistenceEngine>,
    pub metrics:           Arc<RuntimeMetrics>,
    /// Adım 1: REST submit_task için gerçek WASM depolama
    pub module_store:      Arc<ModuleStore>,
    /// Adım 2: Agent execution durumu (in-memory, Faz-2'de persist)
    pub agent_registry:    AgentRegistry,
    /// Adım 3: Workflow execution durumu
    pub workflow_registry: WorkflowRegistry,
    /// Adım 4: Script kayıt ve lookup
    pub script_registry:   Arc<ScriptRegistry>,
    /// Adım B: Cluster state (local/remote dispatch)
    pub cluster:           Arc<ClusterState>,
    /// AI provider registry + aktif provider seçimi.
    /// Sunucu/server dağıtımında ortam değişkenlerinden (ANTHROPIC_API_KEY
    /// vb.) doldurulur — bkz. ApiServer::new() (src/api/mod.rs).
    pub ai_router:         Arc<ProviderRouter>,
}

// ── Router ────────────────────────────────────────────────

pub fn build_router() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(dashboard_handler))
        .route("/metrics",   get(metrics_handler))
        .route("/health",              get(health_handler))
        // Tasks
        .route("/tasks",               post(submit_task_handler))
        .route("/tasks/{id}",          get(task_state_handler))
        .route("/tasks/{id}/cancel",   post(cancel_task_handler))
        // Agents
        .route("/agents",              post(start_agent_handler))
        .route("/agents/{id}",         get(get_agent_handler))
        // Workflows
        .route("/workflows",           post(submit_workflow_handler))
        .route("/workflows/{id}",      get(get_workflow_handler))
        // Cluster / Remote
        .route("/cluster/nodes",    post(register_node_handler))
        .route("/cluster/nodes",    get(list_nodes_handler))
        .route("/cluster/health",   get(cluster_health_handler))
        .route("/remote/command",   post(remote_command_handler))
        // Modules
        .route("/modules",       post(upload_module_handler))
        .route("/modules",       get(list_modules_handler))
        .route("/modules/{hash}", get(get_module_handler))
        // Scripts
        .route("/scripts",      post(run_script_handler))
        .route("/scripts",      get(list_scripts_handler))
        // WebSocket
        .route("/ws", get(crate::api::websocket::ws_upgrade_handler))
}

// ── Handlers ──────────────────────────────────────────────

async fn health_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let metrics = state.metrics.snapshot();
    Json(serde_json::json!({
        "healthy": true,
        "version": env!("CARGO_PKG_VERSION"),
        "active_workers": metrics.active_workers,
    }))
}

async fn metrics_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let s = state.metrics.snapshot();
    Json(serde_json::json!({
        "active_workers":  s.active_workers,
        "queued_tasks":    s.queued_tasks,
        "completed_tasks": s.completed_tasks,
        "failed_tasks":    s.failed_tasks,
        "retried_tasks":   s.retried_tasks,
    }))
}

// ── Task ──────────────────────────────────────────────────

async fn submit_task_handler(
    State(state): State<AppState>,
    Json(req): Json<TaskSubmitRequest>,
) -> impl IntoResponse {
    let task_id = TaskId(Uuid::new_v4());

    // Hex decode
    let wasm_bytes = match hex::decode(&req.wasm_module_hex) {
        Ok(b) => b,
        Err(_) => return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "invalid wasm_module_hex" })),
        ).into_response(),
    };

    // ModuleStore'a kaydet → SHA-256 hash al
    let wasm_module_hash = match state.module_store.store(wasm_bytes) {
        Ok(h) => h,
        Err(e) => return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("module store error: {e:?}") })),
        ).into_response(),
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
        metadata: TaskMetadata { labels: Default::default() },
        wasm_module_hash,
        entrypoint: req.entrypoint,
        state: TaskState::Queued,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    match state.runtime.submit(task).await {
        Ok(()) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "task_id":     task_id.0,
                "status":      "queued",
                "module_hash": hex::encode(wasm_module_hash),
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
        Ok(Some(persisted)) => {
            let task = &persisted.task;
            (StatusCode::OK, Json(serde_json::json!({
                "task_id":    task.id.0,
                "state":      format!("{:?}", task.state),
                "attempts":   persisted.attempts,
                "last_error": persisted.last_error,
                "created_at": task.created_at,
                "updated_at": task.updated_at,
            }))).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "task not found" })),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{e:?}") })),
        ).into_response(),
    }
}

async fn cancel_task_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let task_id = TaskId(id);
    match state.persistence.update_task_state(&task_id, TaskState::Cancelled) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({ "task_id": id, "status": "cancelled" })),
        ).into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "task not found or already terminal" })),
        ).into_response(),
    }
}

// ── Agent ─────────────────────────────────────────────────

async fn start_agent_handler(
    State(state): State<AppState>,
    Json(req): Json<AgentStartRequest>,
) -> impl IntoResponse {
    let execution_id = Uuid::new_v4();
    let agent_id     = Uuid::new_v4();
    let started_at   = Utc::now();
    let objective    = req.objective.clone();

    state.agent_registry.insert(execution_id, AgentStatus {
        execution_id,
        agent_id,
        objective:   objective.clone(),
        status:      "running".into(),
        error:       None,
        started_at,
        finished_at: None,
    });

    let registry = state.agent_registry.clone();
    let context  = AgentContext { agent_id, execution_id, workflow_id: None };
    let budget   = AgentExecutionBudget {
        max_tokens:          req.max_tokens,
        max_steps:           req.max_steps,
        max_runtime_seconds: 300,
    };
    let ai_router = state.ai_router.clone();

    tokio::spawn(async move {
        let result      = AgentExecutor::execute(context, objective, budget, vec![], Some(ai_router)).await;
        let finished_at = Utc::now();
        if let Some(mut entry) = registry.get_mut(&execution_id) {
            match result {
                Ok(_)  => { entry.status = "completed".into(); entry.finished_at = Some(finished_at); }
                Err(e) => { entry.status = "failed".into(); entry.error = Some(format!("{e:?}")); entry.finished_at = Some(finished_at); }
            }
        }
    });

    (StatusCode::ACCEPTED, Json(serde_json::json!({
        "execution_id": execution_id,
        "agent_id":     agent_id,
        "status":       "running",
    }))).into_response()
}

async fn get_agent_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.agent_registry.get(&id) {
        Some(e) => (StatusCode::OK, Json(serde_json::to_value(e.value()).unwrap())).into_response(),
        None    => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "agent not found" }))).into_response(),
    }
}

// ── Workflow ──────────────────────────────────────────────

async fn submit_workflow_handler(
    State(state): State<AppState>,
    Json(req): Json<WorkflowSubmitRequest>,
) -> impl IntoResponse {
    let workflow_id = Uuid::new_v4();
    let started_at  = Utc::now();
    let name        = req.name.clone();

    // Request → WorkflowDsl
    let steps: Vec<StepDsl> = req.steps.into_iter().map(|s| StepDsl {
        id:         s.id,
        name:       s.name,
        kind:       s.kind,
        entrypoint: s.entrypoint,
        depends_on: s.depends_on,
        retryable:  s.retryable,
        labels:     Default::default(),
    }).collect();

    let dsl = WorkflowDsl {
        name:    name.clone(),
        version: Some(1),
        steps,
    };

    state.workflow_registry.insert(workflow_id, WorkflowStatus {
        workflow_id,
        name:       name.clone(),
        status:     "running".into(),
        error:      None,
        started_at,
        finished_at: None,
    });

    let registry    = state.workflow_registry.clone();
    let runtime     = state.runtime.clone();
    let ai_router   = state.ai_router.clone();

    tokio::spawn(async move {
        let result      = WorkflowEngine::run_dsl(&dsl, runtime, ai_router).await;
        let finished_at = Utc::now();
        if let Some(mut entry) = registry.get_mut(&workflow_id) {
            match result {
                Ok(_)  => { entry.status = "completed".into(); entry.finished_at = Some(finished_at); }
                Err(e) => { entry.status = "failed".into(); entry.error = Some(format!("{e:?}")); entry.finished_at = Some(finished_at); }
            }
        }
    });

    (StatusCode::ACCEPTED, Json(serde_json::json!({
        "workflow_id": workflow_id,
        "name":        name,
        "status":      "running",
    }))).into_response()
}

async fn get_workflow_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.workflow_registry.get(&id) {
        Some(e) => (StatusCode::OK, Json(serde_json::to_value(e.value()).unwrap())).into_response(),
        None    => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "workflow not found" }))).into_response(),
    }
}




// ── Cluster / Remote ──────────────────────────────────────

/// Cluster'a yeni node kaydet.
async fn register_node_handler(
    State(state): State<AppState>,
    Json(req): Json<RegisterNodeRequest>,
) -> impl IntoResponse {
    let node = RemoteNode::new(
        req.address.clone(),
        req.capabilities.into_iter().map(|c| match c.as_str() {
            "wasm"     => NodeCapability::WasmExecution,
            "workflow" => NodeCapability::WorkflowExecution,
            "agent"    => NodeCapability::AgentExecution,
            "ai"       => NodeCapability::AiInference,
            "plugin"   => NodeCapability::PluginExecution,
            _          => NodeCapability::WasmExecution,
        }).collect(),
    );

    let node_id = node.node_id;
    state.cluster.register(node);

    (StatusCode::CREATED, Json(serde_json::json!({
        "node_id": node_id,
        "address": req.address,
        "status":  "registered",
    }))).into_response()
}

/// Cluster'daki tüm node'ları listele.
async fn list_nodes_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let nodes: Vec<_> = state.cluster.nodes().into_iter().map(|n| {
        let hb = state.cluster.heartbeat(&n.node_id);
        serde_json::json!({
            "node_id":          n.node_id,
            "address":          n.address,
            "healthy":          n.healthy,
            "last_seen":        n.last_seen,
            "capabilities":     n.capabilities,
            "active_executions": hb.as_ref().map(|h| h.active_executions).unwrap_or(0),
            "cpu_percent":      hb.as_ref().map(|h| h.cpu_usage_percent).unwrap_or(0.0),
            "memory_mb":        hb.as_ref().map(|h| h.memory_usage_mb).unwrap_or(0),
        })
    }).collect();

    (StatusCode::OK, Json(serde_json::json!({
        "count":       nodes.len(),
        "health":      format!("{:?}", state.cluster.health()),
        "has_quorum":  state.cluster.has_quorum(),
        "leader":      state.cluster.leader(),
        "nodes":       nodes,
    }))).into_response()
}

/// Cluster genel sağlık durumu.
async fn cluster_health_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let health      = state.cluster.health();
    let total       = state.cluster.size();
    let healthy     = state.cluster.healthy_count();
    let has_quorum  = state.cluster.has_quorum();
    let leader      = state.cluster.leader();

    let status_code = match health {
        crate::remote::cluster_health::ClusterHealth::Healthy  => StatusCode::OK,
        crate::remote::cluster_health::ClusterHealth::Degraded => StatusCode::OK,
        crate::remote::cluster_health::ClusterHealth::Critical => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(serde_json::json!({
        "health":     format!("{:?}", health),
        "total":      total,
        "healthy":    healthy,
        "has_quorum": has_quorum,
        "leader":     leader,
    }))).into_response()
}

/// Gelen remote komutu işle (diğer node'lardan gelir).
/// POST /remote/command
async fn remote_command_handler(
    State(_state): State<AppState>,
    Json(cmd): Json<RemoteCommand>,
) -> impl IntoResponse {
    match &cmd {
        RemoteCommand::ExecuteTask { task_id } => {
            // Uzaktan gelen task execute isteği → local queue'ya at
            // (task zaten submitting node'da persist edilmiş olmalı)
            tracing::info!(task_id = %task_id, "Remote ExecuteTask received");
            (StatusCode::ACCEPTED, Json(serde_json::json!({
                "status":  "accepted",
                "command": "ExecuteTask",
                "task_id": task_id,
            }))).into_response()
        }
        RemoteCommand::Heartbeat => {
            // Heartbeat ping → pong
            (StatusCode::OK, Json(serde_json::json!({ "status": "pong" }))).into_response()
        }
        RemoteCommand::CancelExecution { execution_id } => {
            tracing::info!(execution_id = %execution_id, "Remote CancelExecution received");
            (StatusCode::ACCEPTED, Json(serde_json::json!({
                "status":       "accepted",
                "command":      "CancelExecution",
                "execution_id": execution_id,
            }))).into_response()
        }
        other => {
            tracing::info!(cmd = ?other, "Remote command received");
            (StatusCode::ACCEPTED, Json(serde_json::json!({
                "status": "accepted",
            }))).into_response()
        }
    }
}

// ── Cluster Request Modelleri ─────────────────────────────

#[derive(serde::Deserialize)]
pub struct RegisterNodeRequest {
    pub address:      String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

// ── Modules ───────────────────────────────────────────────

/// WASM binary'yi hex olarak yükle → hash döndür.
/// FRB'deki upload_wasm_module'ün REST karşılığı.
async fn upload_module_handler(
    State(state): State<AppState>,
    Json(req): Json<ModuleUploadRequest>,
) -> impl IntoResponse {
    let bytes = match hex::decode(&req.wasm_hex) {
        Ok(b) => b,
        Err(_) => return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "invalid wasm_hex" })),
        ).into_response(),
    };

    let size = bytes.len() as u64;

    match state.module_store.store(bytes) {
        Ok(hash) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "hash": hex::encode(hash),
                "size": size,
            })),
        ).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("{e:?}") })),
        ).into_response(),
    }
}

/// Kayıtlı tüm modülleri listele.
async fn list_modules_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let hashes = state.module_store.list_hashes();
    (StatusCode::OK, Json(serde_json::json!({
        "count":   hashes.len(),
        "modules": hashes,
    }))).into_response()
}

/// Belirli bir hash'in varlığını ve boyutunu sorgula.
async fn get_module_handler(
    State(state): State<AppState>,
    Path(hash_hex): Path<String>,
) -> impl IntoResponse {
    let hash = match ModuleStore::hex_to_hash(&hash_hex) {
        Ok(h) => h,
        Err(_) => return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "invalid hash format" })),
        ).into_response(),
    };

    match state.module_store.binary_size(&hash) {
        Some(size) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "hash":   hash_hex,
                "size":   size,
                "exists": true,
            })),
        ).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "module not found" })),
        ).into_response(),
    }
}

// ── Script ────────────────────────────────────────────────

async fn run_script_handler(
    State(state): State<AppState>,
    Json(req): Json<ScriptRunRequest>,
) -> impl IntoResponse {
    // hex → binary
    let wasm_binary = match hex::decode(&req.wasm_hex) {
        Ok(b) => b,
        Err(_) => return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "invalid wasm_hex" })),
        ).into_response(),
    };

    // ModuleStore'a kaydet → hash
    let wasm_module_hash = match state.module_store.store(wasm_binary.clone()) {
        Ok(h) => h,
        Err(e) => return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("module store error: {e:?}") })),
        ).into_response(),
    };

    // ScriptRegistry'ye kaydet
    let script = ScriptDefinition::from_binary(
        req.name.clone(),
        wasm_binary,
        req.entrypoint.clone(),
        req.timeout_ms,
    );
    state.script_registry.register(script);

    // Worker pipeline üzerinden çalıştır
    let task_id = TaskId(Uuid::new_v4());
    let task = TaskDefinition {
        id: task_id,
        parent: None,
        orchestration: None,
        priority: TaskPriority::Normal,
        deadline: None,
        timeout_ms: req.timeout_ms,
        retry_policy: RetryPolicy {
            max_attempts: 1,
            base_delay_ms: 0,
            max_delay_ms: 0,
            jitter: false,
        },
        metadata: TaskMetadata {
            labels: [("script_name".into(), req.name.clone())]
                .into_iter().collect(),
        },
        wasm_module_hash,
        entrypoint: req.entrypoint,
        state: TaskState::Queued,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    match state.runtime.submit(task).await {
        Ok(()) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "task_id":     task_id.0,
                "script_name": req.name,
                "status":      "queued",
                "module_hash": hex::encode(wasm_module_hash),
            })),
        ).into_response(),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response(),
    }
}

async fn list_scripts_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let scripts: Vec<_> = state.script_registry.list()
        .into_iter()
        .map(|s| serde_json::json!({
            "name":        s.name,
            "description": s.description,
            "entrypoint":  s.entrypoint,
            "timeout_ms":  s.timeout_ms,
        }))
        .collect();

    (StatusCode::OK, Json(serde_json::json!({
        "count":   scripts.len(),
        "scripts": scripts,
    }))).into_response()
}

// ── Request Modelleri ─────────────────────────────────────



#[derive(serde::Deserialize)]
pub struct ModuleUploadRequest {
    pub wasm_hex: String,
}

#[derive(serde::Deserialize)]
pub struct ScriptRunRequest {
    pub name:       String,
    pub wasm_hex:   String,
    pub entrypoint: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

#[derive(serde::Deserialize)]
pub struct TaskSubmitRequest {
    pub entrypoint:      String,
    pub wasm_module_hex: String,
    pub priority:        Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms:      u64,
    #[serde(default = "default_attempts")]
    pub max_attempts:    u32,
}
fn default_timeout()  -> u64   { 30_000 }
fn default_attempts() -> u32   { 3 }

#[derive(serde::Deserialize)]
pub struct AgentStartRequest {
    pub objective:  String,
    #[serde(default = "default_steps")]
    pub max_steps:  usize,
    #[serde(default = "default_tokens")]
    pub max_tokens: usize,
}
fn default_steps()  -> usize { 10 }
fn default_tokens() -> usize { 4096 }

#[derive(serde::Deserialize)]
pub struct WorkflowSubmitRequest {
    pub name:  String,
    #[serde(default)]
    pub steps: Vec<WorkflowStepRequest>,
}

#[derive(serde::Deserialize)]
pub struct WorkflowStepRequest {
    pub id:         String,
    pub name:       String,
    #[serde(rename = "type")]
    pub kind:       String,
    pub entrypoint: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub retryable:  bool,
}
