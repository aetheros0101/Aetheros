// ============================================================
// src/tests/runtime_tests.rs
//
// GRUP 4: Runtime Lifecycle Testleri
//
// Test edilen özellikler:
//   - Runtime::new() başarıyla oluşturuluyor
//   - Başlangıç state'i Created (BUG #1 regresyon)
//   - RuntimeConfig::default() geçerli değerler
//   - RuntimeHandle::submit() çalışıyor
//   - RuntimeHandle kanal kapanınca hata döndürüyor
//   - Backpressure: semaphore permit sayısı doğru
//   - Lifecycle state'leri doğru sıralanmış
// ============================================================

use std::time::Duration;

use chrono::Utc;
use tempfile::tempdir;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::runtime::api::RuntimeHandle;
use crate::runtime::backpressure::BackpressureController;
use crate::runtime::config::RuntimeConfig;
use crate::runtime::lifecycle::RuntimeState;
use crate::runtime::runtime::Runtime;
use crate::task::priority::TaskPriority;
use crate::task::retry::RetryPolicy;
use crate::task::task::{TaskDefinition, TaskMetadata, TaskState};
use crate::types::ids::TaskId;

// ── Yardımcılar ───────────────────────────────────────────

fn test_config() -> (RuntimeConfig, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let config = RuntimeConfig {
        worker_count: 1,
        task_channel_capacity: 16,
        event_channel_capacity: 32,
        shutdown_timeout: Duration::from_secs(5),
        max_concurrent_tasks: 8,
        persistence_path: dir.path().to_str().unwrap().to_string(),
    };
    (config, dir)
}

fn make_task() -> TaskDefinition {
    TaskDefinition {
        id: TaskId(Uuid::new_v4()),
        parent: None,
        orchestration: None,
        priority: TaskPriority::Normal,
        deadline: None,
        timeout_ms: 5000,
        retry_policy: RetryPolicy {
            max_attempts: 1,
            base_delay_ms: 0,
            max_delay_ms: 0,
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

// ── Runtime Oluşturma Testleri ────────────────────────────

/// Runtime::new() başarıyla oluşturulmalı.
#[tokio::test]
async fn runtime_creates_successfully() {
    let (config, _dir) = test_config();
    let (_sender, receiver) = mpsc::channel(16);
    let runtime = Runtime::new(config, receiver);
    assert!(runtime.is_ok(), "Runtime oluşturulamamadı");
}

/// BUG #1 regresyon: Başlangıç state'i Created olmalı.
#[tokio::test]
async fn initial_state_is_created() {
    let (config, _dir) = test_config();
    let (_sender, receiver) = mpsc::channel(16);
    let runtime = Runtime::new(config, receiver).unwrap();

    assert_eq!(
        runtime.state(),
        RuntimeState::Created,
        "Başlangıç state Created olmalıydı"
    );
}

/// RuntimeConfig::default() mantıklı değerler içermeli.
#[test]
fn default_config_is_valid() {
    let config = RuntimeConfig::default();
    assert!(config.worker_count > 0);
    assert!(config.task_channel_capacity > 0);
    assert!(config.max_concurrent_tasks > 0);
    assert!(!config.persistence_path.is_empty());
}

// ── RuntimeHandle Testleri ────────────────────────────────

/// RuntimeHandle::submit() kanalı dolu olmadıkça başarılı döner.
#[tokio::test]
async fn handle_submit_succeeds() {
    let (sender, mut receiver) = mpsc::channel(16);
    let handle = RuntimeHandle::new(sender);

    let task = make_task();
    let result = handle.submit(task.clone()).await;
    assert!(result.is_ok(), "Submit başarısız oldu");

    // Kanalda mesaj var mı?
    let received = receiver.try_recv();
    assert!(received.is_ok(), "Kanal boş, task gönderilmedi");
    assert_eq!(received.unwrap().id, task.id);
}

/// Kanal kapanınca submit() hata döndürmeli.
#[tokio::test]
async fn handle_submit_fails_on_closed_channel() {
    let (sender, receiver) = mpsc::channel(1);
    drop(receiver); // Kanalı kapat

    let handle = RuntimeHandle::new(sender);
    let result = handle.submit(make_task()).await;
    assert!(result.is_err(), "Kapalı kanala submit hata döndürmeli");
}

/// Handle clone'lanabilmeli — her ikisi de çalışmalı.
#[tokio::test]
async fn handle_is_cloneable() {
    let (sender, mut receiver) = mpsc::channel(16);
    let handle1 = RuntimeHandle::new(sender);
    let handle2 = handle1.clone();

    handle1.submit(make_task()).await.unwrap();
    handle2.submit(make_task()).await.unwrap();

    // İki mesaj alınabilmeli
    assert!(receiver.try_recv().is_ok());
    assert!(receiver.try_recv().is_ok());
}

// ── Backpressure Testleri ─────────────────────────────────

/// BUG #5 regresyon: Permit sayısı max_concurrent_tasks'a eşit.
#[tokio::test]
async fn backpressure_initial_permits() {
    let bp = BackpressureController::new(4);
    assert_eq!(bp.available(), 4, "Başlangıçta 4 permit olmalı");
}

/// Permit alınınca sayı azalmalı, drop edince artmalı.
#[tokio::test]
async fn backpressure_permit_lifecycle() {
    let bp = BackpressureController::new(2);
    assert_eq!(bp.available(), 2);

    let permit1 = bp.acquire().await;
    assert_eq!(bp.available(), 1);

    let permit2 = bp.acquire().await;
    assert_eq!(bp.available(), 0);

    drop(permit1);
    assert_eq!(bp.available(), 1);

    drop(permit2);
    assert_eq!(bp.available(), 2);
}

// ── Lifecycle State Testleri ──────────────────────────────

/// RuntimeState::from_u8() tüm değerleri doğru map'lemeli.
#[test]
fn lifecycle_state_from_u8() {
    assert_eq!(RuntimeState::from_u8(0), RuntimeState::Created);
    assert_eq!(RuntimeState::from_u8(1), RuntimeState::Starting);
    assert_eq!(RuntimeState::from_u8(2), RuntimeState::Running);
    assert_eq!(RuntimeState::from_u8(3), RuntimeState::Draining);
    assert_eq!(RuntimeState::from_u8(4), RuntimeState::Stopping);
    assert_eq!(RuntimeState::from_u8(5), RuntimeState::Stopped);
    assert_eq!(RuntimeState::from_u8(255), RuntimeState::Failed); // unknown
}

/// is_terminal() yalnızca Stopped ve Failed için true.
#[test]
fn lifecycle_is_terminal() {
    assert!(RuntimeState::Stopped.is_terminal());
    assert!(RuntimeState::Failed.is_terminal());
    assert!(!RuntimeState::Running.is_terminal());
    assert!(!RuntimeState::Draining.is_terminal());
    assert!(!RuntimeState::Created.is_terminal());
}

/// is_running() yalnızca Running ve Draining için true.
#[test]
fn lifecycle_is_running() {
    assert!(RuntimeState::Running.is_running());
    assert!(RuntimeState::Draining.is_running());
    assert!(!RuntimeState::Stopped.is_running());
    assert!(!RuntimeState::Created.is_running());
}
