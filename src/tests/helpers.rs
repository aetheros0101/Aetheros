// ============================================================
// src/tests/helpers.rs  (YENİ — ortak test yardımcısı)
//
// Optimizasyon #1: wasm_module → wasm_module_hash
// Tüm testlerde tekrarlanan make_task() artık buradan gelir.
// wasm_module_hash: [0u8; 32] = "modül yok" (test için)
// ============================================================

use std::collections::HashMap;
use chrono::Utc;
use uuid::Uuid;

use crate::task::priority::TaskPriority;
use crate::task::retry::RetryPolicy;
use crate::task::task::{
    TaskDefinition,
    TaskMetadata,
    TaskState,
};
use crate::types::ids::TaskId;

pub fn make_task(
    priority: TaskPriority,
    entrypoint: &str,
) -> TaskDefinition {
    TaskDefinition {
        id: TaskId(Uuid::new_v4()),
        parent: None,
        orchestration: None,
        priority,
        deadline: None,
        timeout_ms: 5000,
        retry_policy: RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            jitter: false,
        },
        metadata: TaskMetadata {
            labels: HashMap::new(),
        },
        // Optimizasyon #1: hash referansı
        // [0u8; 32] = test stub — modül yok
        wasm_module_hash: [0u8; 32],
        entrypoint: entrypoint.to_string(),
        state: TaskState::Queued,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

pub fn make_task_with_hash(
    priority: TaskPriority,
    hash: [u8; 32],
) -> TaskDefinition {
    let mut task = make_task(priority, "main");
    task.wasm_module_hash = hash;
    task
}
