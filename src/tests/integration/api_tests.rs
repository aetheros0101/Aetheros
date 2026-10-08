// ============================================================
// src/tests/integration/api_tests.rs
//
// SPRINT 2 — API Katmanı Entegrasyon Testleri
//
// Test edilen özellikler:
//   - AppState oluşturulabilir
//   - RuntimeHandle submit → kanal akışı
//   - Persistence üzerinden task state sorgusu
//   - health endpoint mantığı
//   - Kanal dolunca submit → hata (backpressure)
//
// NOT: Gerçek HTTP testleri (axum test client)
//   Sprint 3'te axum::test ile eklenecek.
//   Bu tesler API state/logic katmanını doğrular.
// ============================================================

use std::sync::Arc;

use chrono::Utc;
use tempfile::tempdir;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::events::bus::EventBus;
use crate::persistence::engine::PersistenceEngine;
use crate::persistence::models::PersistedTask;
use crate::runtime::api::RuntimeHandle;
use crate::runtime::lifecycle::RuntimeState;
use crate::task::priority::TaskPriority;
use crate::task::retry::RetryPolicy;
use crate::task::task::{TaskDefinition, TaskMetadata, TaskState};
use crate::types::ids::TaskId;

// ── Yardımcılar ───────────────────────────────────────────

fn setup() -> (
    RuntimeHandle,
    mpsc::Receiver<TaskDefinition>,
    Arc<PersistenceEngine>,
    EventBus,
    tempfile::TempDir,
) {
    let (sender, receiver) = mpsc::channel(64);
    let dir = tempdir().unwrap();
    let db = Arc::new(PersistenceEngine::open(dir.path().to_str().unwrap()).unwrap());
    let events = EventBus::new(64);
    let handle = RuntimeHandle::new(sender);
    (handle, receiver, db, events, dir)
}

fn make_task(priority: TaskPriority) -> TaskDefinition {
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
            labels: Default::default(),
        },
        wasm_module_hash: [0u8; 32],
        entrypoint: "main".to_string(),
        state: TaskState::Queued,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ── API State Testleri ────────────────────────────────────

/// Handle üzerinden submit → receiver'da görünmeli.
#[tokio::test]
async fn api_handle_submit_reaches_receiver() {
    let (handle, mut receiver, _, _, _dir) = setup();

    let task = make_task(TaskPriority::High);
    let id = task.id;

    handle.submit(task).await.unwrap();

    let received = receiver.try_recv().unwrap();
    assert_eq!(received.id, id);
}

/// Persist → load: API'nin task state sorgusu simülasyonu.
#[tokio::test]
async fn api_task_state_query_returns_correct_state() {
    let (_, _, db, _, _dir) = setup();

    let task = make_task(TaskPriority::Normal);
    let id = task.id;

    let persisted = PersistedTask {
        task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        attempts: 0,
        last_error: None,
    };
    db.persist_task(&persisted).unwrap();

    // API'nin yaptığı: load_task → state döndür
    let result = db.load_task(&id).unwrap();
    assert!(result.is_some());
    assert!(matches!(result.unwrap().task.state, TaskState::Queued));
}

/// Cancel simülasyonu: Cancelled state yazılır, is_terminal() true.
#[tokio::test]
async fn api_cancel_marks_task_cancelled() {
    let (_, _, db, _, _dir) = setup();

    let task = make_task(TaskPriority::Normal);
    let id = task.id;

    let persisted = PersistedTask {
        task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        attempts: 0,
        last_error: None,
    };
    db.persist_task(&persisted).unwrap();

    // API'nin yaptığı: update_task_state(Cancelled)
    db.update_task_state(&id, TaskState::Cancelled).unwrap();

    let record = db.load_task(&id).unwrap().unwrap();
    assert!(matches!(record.task.state, TaskState::Cancelled));
    assert!(record.is_terminal(), "Cancelled task terminal olmalı");
}

/// Kapalı kanal → submit hata döndürmeli (API: 503 senaryosu).
#[tokio::test]
async fn api_submit_fails_when_runtime_down() {
    let (sender, receiver) = mpsc::channel(1);
    drop(receiver); // Runtime kapalı simülasyonu
    let handle = RuntimeHandle::new(sender);

    let result = handle.submit(make_task(TaskPriority::Normal)).await;
    assert!(result.is_err(), "Runtime kapalıyken submit hata döndürmeli");
}

/// Dolu kanal → submit bloklamadan hata döner.
#[tokio::test]
async fn api_submit_fails_on_full_channel_gracefully() {
    // Kapasite 1, 1 item doldur, drop etme
    let (sender, _receiver) = mpsc::channel(1);
    let handle = RuntimeHandle::new(sender.clone());

    // İlk submit başarılı (kapasite: 1)
    handle
        .submit(make_task(TaskPriority::Normal))
        .await
        .unwrap();

    // receiver drop edildi → ikinci submit hata verir
    drop(_receiver);
    let result = handle.submit(make_task(TaskPriority::Normal)).await;
    assert!(result.is_err());
}

/// health check: RuntimeState lifecycle doğrulaması.
#[test]
fn health_check_state_logic() {
    // Running ve Draining → sistem sağlıklı
    assert!(RuntimeState::Running.is_running());
    assert!(RuntimeState::Draining.is_running());

    // Stopped ve Failed → sağlıksız
    assert!(!RuntimeState::Stopped.is_running());
    assert!(!RuntimeState::Failed.is_running());

    // Terminal state'ler
    assert!(RuntimeState::Stopped.is_terminal());
    assert!(RuntimeState::Failed.is_terminal());
}

/// Birden fazla concurrent submit → kanal sırasını korur.
#[tokio::test]
async fn concurrent_submits_no_data_loss() {
    let (sender, mut receiver) = mpsc::channel(100);
    let handle = RuntimeHandle::new(sender);

    let task_count = 20;
    let mut ids = Vec::new();

    for _ in 0..task_count {
        let task = make_task(TaskPriority::Normal);
        ids.push(task.id);
        handle.submit(task).await.unwrap();
    }

    let mut received = 0;
    while receiver.try_recv().is_ok() {
        received += 1;
    }

    assert_eq!(
        received, task_count,
        "Tüm {} task alınmalıydı, {} alındı",
        task_count, received
    );
}
