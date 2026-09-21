// ============================================================
// src/tests/persistence_tests.rs
//
// GRUP 3: Persistence Testleri
//
// Test edilen özellikler:
//   - persist → load round-trip
//   - update_task_state() her state için
//   - is_terminal(): doğru state'ler
//   - Recovery: terminal task'lar filtreleniyor (BUG #4 reg.)
//   - Recovery: attempts limiti aşılanlar filtreleniyor
//   - delete_task() çalışıyor
//   - load_all_tasks() tüm kayıtları döndürüyor
// ============================================================

use chrono::Utc;
use tempfile::tempdir;
use uuid::Uuid;

use crate::persistence::engine::PersistenceEngine;
use crate::persistence::models::PersistedTask;
use crate::persistence::recovery::RecoveryEngine;
use crate::task::priority::TaskPriority;
use crate::task::retry::RetryPolicy;
use crate::task::task::{
    TaskDefinition,
    TaskMetadata,
    TaskState,
};
use crate::types::ids::TaskId;

// ── Yardımcılar ───────────────────────────────────────────

fn test_db() -> (PersistenceEngine, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let db = PersistenceEngine::open(
        dir.path().to_str().unwrap(),
    )
    .unwrap();
    (db, dir)
}

fn make_persisted(state: TaskState, attempts: u32) -> PersistedTask {
    let task = TaskDefinition {
        id: TaskId(Uuid::new_v4()),
        parent: None,
        orchestration: None,
        priority: TaskPriority::Normal,
        deadline: None,
        timeout_ms: 5000,
        retry_policy: RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            jitter: false,
        },
        metadata: TaskMetadata {
            labels: Default::default(),
        },
        wasm_module_hash: [0u8; 32],
        entrypoint: "main".to_string(),
        state,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    PersistedTask {
        task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        attempts,
        last_error: None,
    }
}

// ── CRUD Testleri ─────────────────────────────────────────

#[test]
fn persist_and_load_roundtrip() {
    let (db, _dir) = test_db();
    let persisted = make_persisted(TaskState::Queued, 0);
    let id = persisted.task.id;

    db.persist_task(&persisted).unwrap();

    let loaded = db.load_task(&id).unwrap();
    assert!(loaded.is_some(), "Task yüklenemedi");
    let loaded = loaded.unwrap();
    assert_eq!(loaded.task.id, id);
    assert_eq!(loaded.attempts, 0);
}

#[test]
fn load_nonexistent_returns_none() {
    let (db, _dir) = test_db();
    let fake_id = TaskId(Uuid::new_v4());
    let result = db.load_task(&fake_id).unwrap();
    assert!(result.is_none(), "Olmayan task None döndürmeli");
}

#[test]
fn delete_task_removes_it() {
    let (db, _dir) = test_db();
    let persisted = make_persisted(TaskState::Queued, 0);
    let id = persisted.task.id;

    db.persist_task(&persisted).unwrap();
    assert!(db.load_task(&id).unwrap().is_some());

    db.delete_task(&id).unwrap();
    assert!(
        db.load_task(&id).unwrap().is_none(),
        "Silinen task bulunamamalı"
    );
}

#[test]
fn load_all_returns_all_tasks() {
    let (db, _dir) = test_db();

    for _ in 0..5 {
        let p = make_persisted(TaskState::Queued, 0);
        db.persist_task(&p).unwrap();
    }

    let all = db.load_all_tasks().unwrap();
    assert_eq!(all.len(), 5, "5 task kayıtlı olmalı");
}

// ── State Güncelleme Testleri ─────────────────────────────

/// BUG #4 regresyon: Her state geçişi kalıcı olmalı.
#[test]
fn update_state_executing() {
    let (db, _dir) = test_db();
    let persisted = make_persisted(TaskState::Queued, 0);
    let id = persisted.task.id;

    db.persist_task(&persisted).unwrap();
    db.update_task_state(&id, TaskState::Executing).unwrap();

    let loaded = db.load_task(&id).unwrap().unwrap();
    assert!(
        matches!(loaded.task.state, TaskState::Executing),
        "State Executing olmalıydı"
    );
}

#[test]
fn update_state_completed() {
    let (db, _dir) = test_db();
    let persisted = make_persisted(TaskState::Executing, 0);
    let id = persisted.task.id;

    db.persist_task(&persisted).unwrap();
    db.update_task_state(&id, TaskState::Completed).unwrap();

    let loaded = db.load_task(&id).unwrap().unwrap();
    assert!(
        matches!(loaded.task.state, TaskState::Completed),
        "State Completed olmalıydı"
    );
}

#[test]
fn update_state_failed() {
    let (db, _dir) = test_db();
    let persisted = make_persisted(TaskState::Executing, 0);
    let id = persisted.task.id;

    db.persist_task(&persisted).unwrap();
    db.update_task_state(&id, TaskState::Failed).unwrap();

    let loaded = db.load_task(&id).unwrap().unwrap();
    assert!(
        matches!(loaded.task.state, TaskState::Failed)
    );
}

// ── is_terminal() Testleri ────────────────────────────────

#[test]
fn completed_is_terminal() {
    let p = make_persisted(TaskState::Completed, 0);
    assert!(p.is_terminal());
}

#[test]
fn failed_is_terminal() {
    let p = make_persisted(TaskState::Failed, 0);
    assert!(p.is_terminal());
}

#[test]
fn cancelled_is_terminal() {
    let p = make_persisted(TaskState::Cancelled, 0);
    assert!(p.is_terminal());
}

#[test]
fn queued_is_not_terminal() {
    let p = make_persisted(TaskState::Queued, 0);
    assert!(!p.is_terminal());
}

#[test]
fn executing_is_not_terminal() {
    let p = make_persisted(TaskState::Executing, 0);
    assert!(!p.is_terminal());
}

// ── Recovery Testleri ─────────────────────────────────────

/// BUG #4 regresyon: Completed task'lar recovery'e girmemeli.
#[test]
fn recovery_skips_terminal_tasks() {
    let tasks = vec![
        make_persisted(TaskState::Completed, 0),  // skip
        make_persisted(TaskState::Failed, 2),      // skip
        make_persisted(TaskState::Cancelled, 0),   // skip
        make_persisted(TaskState::Queued, 0),      // dahil
        make_persisted(TaskState::Executing, 1),   // dahil
    ];

    let recoverable = RecoveryEngine::recoverable_tasks(tasks);
    assert_eq!(
        recoverable.len(), 2,
        "Sadece non-terminal task'lar recovery'e girmeli"
    );
}

/// Attempts limiti aşılanlar da recovery'e girmemeli.
#[test]
fn recovery_skips_exhausted_attempts() {
    let mut exhausted = make_persisted(TaskState::Queued, 3);
    exhausted.task.retry_policy.max_attempts = 3;

    let tasks = vec![
        exhausted,                                  // skip (3 >= 3)
        make_persisted(TaskState::Queued, 0),       // dahil
    ];

    let recoverable = RecoveryEngine::recoverable_tasks(tasks);
    assert_eq!(recoverable.len(), 1);
}

/// mark_recovered() state'i Queued yapmalı.
#[test]
fn mark_recovered_sets_queued() {
    let mut task = make_persisted(TaskState::Executing, 1);
    RecoveryEngine::mark_recovered(&mut task);
    assert!(
        matches!(task.task.state, TaskState::Queued),
        "mark_recovered sonrası state Queued olmalı"
    );
}
