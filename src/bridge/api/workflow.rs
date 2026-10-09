// ============================================================
// bridge/api/workflow.rs
//
// Workflow başlatma, durum ve listeleme.
// ============================================================

use crate::api::rest::router::WorkflowStepRequest;
use crate::bridge::api::error::require_runtime;
use crate::bridge::types::{WorkflowStartResponse, WorkflowStatusResponse};

// ── Workflow fonksiyonları (FRB) ──────────────────────────────

/// Workflow başlat → workflow_id döner.
pub async fn start_workflow(
    name: String,
    steps: Vec<WorkflowStepRequest>,
) -> Result<WorkflowStartResponse, String> {
    let rt = require_runtime()?;

    let workflow_id = uuid::Uuid::new_v4();
    let started_at = chrono::Utc::now();
    let name_c = name.clone();

    use crate::workflows::compiler::{StepDsl, WorkflowDsl};
    let dsl_steps: Vec<StepDsl> = steps
        .into_iter()
        .map(|s| StepDsl {
            id: s.id,
            name: s.name,
            kind: s.kind,
            entrypoint: s.entrypoint,
            depends_on: s.depends_on,
            retryable: s.retryable,
            labels: Default::default(),
        })
        .collect();

    let dsl = WorkflowDsl {
        name: name.clone(),
        version: Some(1),
        steps: dsl_steps,
    };

    rt.workflow_registry.insert(
        workflow_id,
        crate::bridge::agent::WorkflowEntry {
            workflow_id,
            name: name.clone(),
            status: "running".into(),
            error: None,
            started_at,
            finished_at: None,
        },
    );

    let registry = rt.workflow_registry.clone();
    let runtime = rt.handle.clone();
    let ai_router = rt.ai_router.clone();

    tokio::spawn(async move {
        let result =
            crate::workflows::engine::WorkflowEngine::run_dsl(&dsl, runtime, ai_router).await;
        let finished_at = chrono::Utc::now();
        if let Some(mut entry) = registry.get_mut(&workflow_id) {
            match result {
                Ok(_) => {
                    entry.status = "completed".into();
                    entry.finished_at = Some(finished_at);
                }
                Err(e) => {
                    entry.status = "failed".into();
                    entry.error = Some(format!("{e:?}"));
                    entry.finished_at = Some(finished_at);
                }
            }
        }
    });

    Ok(WorkflowStartResponse {
        workflow_id: workflow_id.to_string(),
        name: name_c,
        status: "running".into(),
    })
}

/// Workflow durumu sorgula.
pub fn get_workflow_status(workflow_id: String) -> Result<WorkflowStatusResponse, String> {
    let rt = require_runtime()?;

    let uuid = uuid::Uuid::parse_str(&workflow_id)
        .map_err(|_| format!("Geçersiz workflow_id: {workflow_id}"))?;

    rt.workflow_registry
        .get(&uuid)
        .map(|e| WorkflowStatusResponse {
            workflow_id: e.workflow_id.to_string(),
            name: e.name.clone(),
            status: e.status.clone(),
            error: e.error.clone(),
            started_at: e.started_at.timestamp_millis(),
            finished_at: e.finished_at.map(|t| t.timestamp_millis()),
        })
        .ok_or_else(|| format!("Workflow bulunamadı: {workflow_id}"))
}

/// Tüm workflow'ları listele.
pub fn list_workflows(limit: usize) -> Result<Vec<WorkflowStatusResponse>, String> {
    let rt = require_runtime()?;

    let mut list: Vec<WorkflowStatusResponse> = rt
        .workflow_registry
        .iter()
        .take(limit)
        .map(|e| WorkflowStatusResponse {
            workflow_id: e.workflow_id.to_string(),
            name: e.name.clone(),
            status: e.status.clone(),
            error: e.error.clone(),
            started_at: e.started_at.timestamp_millis(),
            finished_at: e.finished_at.map(|t| t.timestamp_millis()),
        })
        .collect();

    list.sort_by_key(|e| std::cmp::Reverse(e.started_at));
    Ok(list)
}
