// ============================================================
// bridge/api/task.rs
//
// Task yönetimi: gönderme, durum sorgulama, listeleme, yeniden gönderme.
// Sorumluluk: Task lifecycle API'si.
// ============================================================

use tracing::info;

use crate::bridge::api::error::require_runtime;
use crate::bridge::api::helpers::{parse_hash, parse_priority};
use crate::bridge::types::{TaskRequest, TaskStatusResponse};
use crate::task::retry::RetryPolicy;
use crate::task::task::{TaskDefinition, TaskMetadata, TaskState};
use crate::types::ids::TaskId;

// ── Task yönetimi ─────────────────────────────────────────

/// Task gönder → task_id döner.
///
/// WASM modülü yoksa (wasm_module_hash == "") task yine kabul
/// edilir; runtime Failed olarak işaretler.
///
/// Örnek (Dart):
/// ```dart
/// final taskId = await AetherApi.submitTask(TaskRequest(
///   wasmModuleHash: hash,
///   entrypoint: "run",
///   priority: "normal",
///   timeoutMs: 30000,
///   maxRetries: 3,
/// ));
/// ```
pub async fn submit_task(request: TaskRequest) -> Result<String, String> {
    let rt = require_runtime()?;

    // priority string → TaskPriority
    let priority = parse_priority(&request.priority)?;

    // wasm_module_hash: hex → [u8;32]
    let wasm_module_hash = parse_hash(&request.wasm_module_hash)?;

    let timeout_ms = if request.timeout_ms == 0 {
        30_000
    } else {
        request.timeout_ms
    };

    let task_id = TaskId(uuid::Uuid::new_v4());

    let now = chrono::Utc::now();

    let task = TaskDefinition {
        id: task_id,
        parent: None,
        orchestration: None,
        priority,
        deadline: None,
        timeout_ms,
        retry_policy: RetryPolicy {
            max_attempts: request.max_retries.max(1),
            base_delay_ms: 100,
            max_delay_ms: 10_000,
            jitter: true,
        },
        metadata: TaskMetadata {
            labels: std::collections::HashMap::new(),
        },
        wasm_module_hash,
        entrypoint: request.entrypoint,
        state: TaskState::Created,
        created_at: now,
        updated_at: now,
    };

    rt.handle
        .submit(task)
        .await
        .map_err(|e| format!("Submit hatası: {e:?}"))?;

    info!(task_id = %task_id.0, "Task gönderildi");

    Ok(task_id.0.to_string())
}

/// Task durumunu sorgula.
pub async fn get_task_status(task_id: String) -> Result<TaskStatusResponse, String> {
    let rt = require_runtime()?;

    let uuid =
        uuid::Uuid::parse_str(&task_id).map_err(|_| format!("Geçersiz task_id: {task_id}"))?;

    let tid = TaskId(uuid);

    let persisted = rt
        .persistence
        .load_task(&tid)
        .map_err(|e| format!("Persistence hatası: {e:?}"))?
        .ok_or_else(|| format!("Task bulunamadı: {task_id}"))?;

    let state_str = format!("{:?}", persisted.task.state);

    // Gerçek hata mesajı persistence'tan (worker.fail_task ile
    // kaydedildi). Eski kayıtlarda last_error=None olabilir —
    // bu durumda genel bir mesaja düş.
    let error_message = match &persisted.task.state {
        crate::task::task::TaskState::Failed => Some(
            persisted
                .last_error
                .clone()
                .unwrap_or_else(|| "WASM yürütme başarısız (detay yok)".to_string()),
        ),
        _ => None,
    };

    Ok(TaskStatusResponse {
        task_id,
        state: state_str,
        created_at: persisted.created_at.timestamp_millis(),
        updated_at: persisted.updated_at.timestamp_millis(),
        attempts: persisted.attempts,
        error_message,
    })
}

/// Tüm task'ları listele (son N tane).
pub async fn list_tasks(limit: u32) -> Result<Vec<TaskStatusResponse>, String> {
    let rt = require_runtime()?;

    let all = match rt.persistence.load_all_tasks() {
        Ok(tasks) => tasks,
        Err(e) => {
            // Eski format kayıtlar MessagePack deserialize edemeyebilir.
            // Yutmak yerine logla — UI'da boş liste yerine hata göster.
            tracing::warn!(err = ?e, "load_all_tasks kısmen başarısız");
            return Err(format!("Task listesi yüklenemedi: {e:?}"));
        }
    };

    let responses: Vec<TaskStatusResponse> = all
        .into_iter()
        .rev() // En yeni önce
        .take(limit as usize)
        .map(|p| {
            let state_str = format!("{:?}", p.task.state);
            let error_message = match &p.task.state {
                crate::task::task::TaskState::Failed => Some(
                    p.last_error
                        .clone()
                        .unwrap_or_else(|| "WASM yürütme başarısız (detay yok)".to_string()),
                ),
                _ => None,
            };
            TaskStatusResponse {
                task_id: p.task.id.0.to_string(),
                state: state_str,
                created_at: p.created_at.timestamp_millis(),
                updated_at: p.updated_at.timestamp_millis(),
                attempts: p.attempts,
                error_message,
            }
        })
        .collect();

    Ok(responses)
}

pub async fn resubmit_task(task_id: String) -> Result<String, String> {
    let rt = require_runtime()?;

    let uuid =
        uuid::Uuid::parse_str(&task_id).map_err(|_| format!("Geçersiz task_id: {task_id}"))?;

    let original = rt
        .persistence
        .load_task(&TaskId(uuid))
        .map_err(|e| format!("Persistence hatası: {e:?}"))?
        .ok_or_else(|| format!("Task bulunamadı: {task_id}"))?
        .task;

    let new_id = TaskId(uuid::Uuid::new_v4());
    let now = chrono::Utc::now();

    let resubmitted = TaskDefinition {
        id: new_id,
        parent: original.parent,
        orchestration: original.orchestration,
        priority: original.priority,
        deadline: None,
        timeout_ms: original.timeout_ms,
        retry_policy: original.retry_policy,
        metadata: original.metadata,
        wasm_module_hash: original.wasm_module_hash,
        entrypoint: original.entrypoint,
        state: TaskState::Created,
        created_at: now,
        updated_at: now,
    };

    rt.handle
        .submit(resubmitted)
        .await
        .map_err(|e| format!("Submit hatası: {e:?}"))?;

    info!(old_task_id = %task_id, new_task_id = %new_id.0, "Task yeniden gönderildi");

    Ok(new_id.0.to_string())
}
