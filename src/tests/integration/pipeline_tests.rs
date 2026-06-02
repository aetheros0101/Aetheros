// ============================================================
// src/tests/integration/pipeline_tests.rs
//
// SPRINT 2 — Uçtan Uca Pipeline Testleri
//
// Test edilen senaryo:
//   RuntimeHandle::submit() → mpsc kanal → scheduler
//   → PriorityTaskQueue → Dispatcher → Worker
//
// NOT: Gerçek WASM execution bu testlerde çalışmaz
//   (binary yok). Bunun yerine:
//   - Submit → kanal akışı test edilir
//   - Priority sırası kanal üzerinden doğrulanır
//   - Persistence round-trip doğrulanır
//   - EventBus üzerinden task event'leri izlenir
//
// Gerçek WASM entegrasyon testi Sprint 3'te
// (minimal WAT binary ile).
// ============================================================

use std::time::Duration;

use chrono::Utc;
use tempfile::tempdir;
use tokio::sync::mpsc;
use tokio::time::timeout;
use uuid::Uuid;


use crate::persistence::engine::PersistenceEngine;
use crate::persistence::models::PersistedTask;
use crate::runtime::api::RuntimeHandle;
use crate::task::priority::TaskPriority;
use crate::task::queue::PriorityTaskQueue;
use crate::task::retry::RetryPolicy;
use crate::task::task::{
    TaskDefinition,
    TaskMetadata,
    TaskState,
};
use crate::types::ids::TaskId;

// ── Yardımcılar ───────────────────────────────────────────

fn make_task(
    priority: TaskPriority,
    label: &str,
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
            labels: [("label".to_string(), label.to_string())]
                .into_iter()
                .collect(),
        },
        wasm_module_hash: [0u8; 32],
        entrypoint: label.to_string(),
        state: TaskState::Queued,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ── Pipeline Testleri ─────────────────────────────────────

/// Submit → kanal → alıcı taraf doğru task'ı alıyor.
#[tokio::test]
async fn submit_reaches_receiver() {
    let (sender, mut receiver) = mpsc::channel(16);
    let handle = RuntimeHandle::new(sender);

    let task = make_task(TaskPriority::High, "test-task");
    let task_id = task.id;

    handle.submit(task).await.unwrap();

    let received = timeout(
        Duration::from_millis(100),
        receiver.recv(),
    )
    .await
    .expect("timeout")
    .expect("channel closed");

    assert_eq!(
        received.id, task_id,
        "Alınan task ID eşleşmeli"
    );
}

/// Birden fazla submit → kanal sırasıyla teslim eder.
#[tokio::test]
async fn multiple_submits_ordered_in_channel() {
    let (sender, mut receiver) = mpsc::channel(16);
    let handle = RuntimeHandle::new(sender);

    let tasks: Vec<TaskDefinition> = (0..5)
        .map(|i| make_task(
            TaskPriority::Normal,
            &format!("task-{}", i),
        ))
        .collect();

    let ids: Vec<TaskId> =
        tasks.iter().map(|t| t.id).collect();

    for task in tasks {
        handle.submit(task).await.unwrap();
    }

    for expected_id in ids {
        let received = receiver.try_recv().unwrap();
        assert_eq!(received.id, expected_id);
    }
}

/// Queue → priority sırası: Critical önce çıkmalı.
#[tokio::test]
async fn queue_priority_preserved_end_to_end() {
    let queue = PriorityTaskQueue::new();

    // Ters sırada ekle
    queue.push(make_task(TaskPriority::Low, "low")).await.unwrap();
    queue.push(make_task(TaskPriority::Normal, "normal")).await.unwrap();
    queue.push(make_task(TaskPriority::Critical, "critical")).await.unwrap();
    queue.push(make_task(TaskPriority::High, "high")).await.unwrap();

    let order = vec![
        queue.pop().await.unwrap().entrypoint,
        queue.pop().await.unwrap().entrypoint,
        queue.pop().await.unwrap().entrypoint,
        queue.pop().await.unwrap().entrypoint,
    ];

    assert_eq!(
        order,
        vec!["critical", "high", "normal", "low"],
        "Kuyruktan çıkış sırası yanlış: {:?}", order
    );
}

/// Submit → persist → load: task DB'de görünmeli.
#[tokio::test]
async fn submitted_task_persists_to_db() {
    let dir = tempdir().unwrap();
    let db = PersistenceEngine::open(
        dir.path().to_str().unwrap(),
    )
    .unwrap();

    let task = make_task(TaskPriority::Normal, "persist-test");
    let task_id = task.id;

    // Runtime'ın yaptığı gibi persist et
    let persisted = PersistedTask {
        task: task.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        attempts: 0,
    };
    db.persist_task(&persisted).unwrap();

    // Yükle ve doğrula
    let loaded = db.load_task(&task_id).unwrap();
    assert!(loaded.is_some(), "Task DB'de bulunmalı");

    let loaded = loaded.unwrap();
    assert_eq!(loaded.task.id, task_id);
    assert!(matches!(
        loaded.task.state,
        TaskState::Queued
    ));
}

/// Persist → state update → load: lifecycle doğrulanır.
#[tokio::test]
async fn task_state_lifecycle_persists_correctly() {
    let dir = tempdir().unwrap();
    let db = PersistenceEngine::open(
        dir.path().to_str().unwrap(),
    )
    .unwrap();

    let task = make_task(TaskPriority::High, "lifecycle");
    let id = task.id;

    // Queued
    let persisted = PersistedTask {
        task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        attempts: 0,
    };
    db.persist_task(&persisted).unwrap();

    // Executing
    db.update_task_state(&id, TaskState::Executing).unwrap();
    let state = db.load_task(&id).unwrap().unwrap().task.state;
    assert!(matches!(state, TaskState::Executing));

    // Completed
    db.update_task_state(&id, TaskState::Completed).unwrap();
    let state = db.load_task(&id).unwrap().unwrap().task.state;
    assert!(matches!(state, TaskState::Completed));

    // is_terminal() kontrolü
    let final_record = db.load_task(&id).unwrap().unwrap();
    assert!(final_record.is_terminal());
}

/// Deadline geçmiş task: deadline_expired() true dönmeli.
#[tokio::test]
async fn expired_deadline_detected() {
    use crate::task::deadline::deadline_expired;

    let mut task = make_task(TaskPriority::Normal, "deadline");
    // Geçmişe deadline ver
    task.deadline = Some(
        Utc::now() - chrono::Duration::seconds(10),
    );

    assert!(
        deadline_expired(&task),
        "Geçmiş deadline tespit edilmeli"
    );
}

/// Gelecek deadline: deadline_expired() false dönmeli.
#[tokio::test]
async fn future_deadline_not_expired() {
    use crate::task::deadline::deadline_expired;

    let mut task = make_task(TaskPriority::Normal, "future");
    task.deadline = Some(
        Utc::now() + chrono::Duration::seconds(60),
    );

    assert!(
        !deadline_expired(&task),
        "Gelecek deadline expired sayılmamalı"
    );
}
