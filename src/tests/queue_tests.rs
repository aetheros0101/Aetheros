// ============================================================
// src/tests/queue_tests.rs
//
// GRUP 1: Priority Queue Testleri
//
// Test edilen özellikler:
//   - Critical task Low'dan önce çıkar (BUG #2 regresyon)
//   - Tüm 4 öncelik doğru sıralanır
//   - Aynı öncelikte FIFO korunur
//   - Boş kuyrukta pop() bloklar
//   - Len ve is_empty doğru çalışır
// ============================================================

use crate::task::priority::TaskPriority;
use crate::task::queue::PriorityTaskQueue;
use crate::task::task::{
    TaskDefinition,
    TaskMetadata,
    TaskState,
};
use crate::task::retry::RetryPolicy;
use crate::types::ids::TaskId;

use chrono::Utc;
use uuid::Uuid;

// ── Yardımcı fonksiyon ────────────────────────────────────

fn make_task(
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
            max_attempts: 1,
            base_delay_ms: 0,
            max_delay_ms: 0,
            jitter: false,
        },
        metadata: TaskMetadata {
            labels: Default::default(),
        },
        wasm_module_hash: [0u8; 32],
        entrypoint: entrypoint.to_string(),
        state: TaskState::Queued,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ── Testler ───────────────────────────────────────────────

/// BUG #2 regresyon: Critical Low'dan önce çıkmalı.
#[tokio::test]
async fn critical_pops_before_low() {
    let q = PriorityTaskQueue::new();

    // Önce Low, sonra Critical ekle (ters sıra)
    q.push(make_task(TaskPriority::Low, "low"))
        .await
        .unwrap();
    q.push(make_task(TaskPriority::Critical, "critical"))
        .await
        .unwrap();

    let first = q.pop().await.unwrap();
    assert_eq!(
        first.entrypoint, "critical",
        "Critical task Low'dan önce çıkmalıydı"
    );

    let second = q.pop().await.unwrap();
    assert_eq!(second.entrypoint, "low");
}

/// Tüm 4 seviye doğru sırayla çıkmalı.
#[tokio::test]
async fn all_priorities_correct_order() {
    let q = PriorityTaskQueue::new();

    // Rastgele sırada ekle
    q.push(make_task(TaskPriority::Normal, "normal")).await.unwrap();
    q.push(make_task(TaskPriority::Low, "low")).await.unwrap();
    q.push(make_task(TaskPriority::Critical, "critical")).await.unwrap();
    q.push(make_task(TaskPriority::High, "high")).await.unwrap();

    let order: Vec<String> = vec![
        q.pop().await.unwrap().entrypoint,
        q.pop().await.unwrap().entrypoint,
        q.pop().await.unwrap().entrypoint,
        q.pop().await.unwrap().entrypoint,
    ];

    assert_eq!(
        order,
        vec!["critical", "high", "normal", "low"],
        "Öncelik sırası yanlış: {:?}", order
    );
}

/// Aynı öncelikte FIFO korunmalı.
#[tokio::test]
async fn same_priority_fifo() {
    let q = PriorityTaskQueue::new();

    q.push(make_task(TaskPriority::Normal, "first")).await.unwrap();
    q.push(make_task(TaskPriority::Normal, "second")).await.unwrap();
    q.push(make_task(TaskPriority::Normal, "third")).await.unwrap();

    assert_eq!(q.pop().await.unwrap().entrypoint, "first");
    assert_eq!(q.pop().await.unwrap().entrypoint, "second");
    assert_eq!(q.pop().await.unwrap().entrypoint, "third");
}

/// len() ve is_empty() doğru çalışmalı.
#[tokio::test]
async fn len_and_is_empty() {
    let q = PriorityTaskQueue::new();

    assert!(q.is_empty().await);
    assert_eq!(q.len().await, 0);

    q.push(make_task(TaskPriority::Normal, "a")).await.unwrap();
    q.push(make_task(TaskPriority::Normal, "b")).await.unwrap();

    assert!(!q.is_empty().await);
    assert_eq!(q.len().await, 2);

    q.pop().await.unwrap();
    assert_eq!(q.len().await, 1);

    q.pop().await.unwrap();
    assert!(q.is_empty().await);
}

/// Mixed öncelik + FIFO: aynı öncelikli grup içinde sıra korunur,
/// farklı gruplar arası öncelik geçerli.
#[tokio::test]
async fn mixed_priority_and_fifo() {
    let q = PriorityTaskQueue::new();

    q.push(make_task(TaskPriority::Low, "low-1")).await.unwrap();
    q.push(make_task(TaskPriority::High, "high-1")).await.unwrap();
    q.push(make_task(TaskPriority::Low, "low-2")).await.unwrap();
    q.push(make_task(TaskPriority::High, "high-2")).await.unwrap();

    // high'lar önce, kendi aralarında FIFO
    assert_eq!(q.pop().await.unwrap().entrypoint, "high-1");
    assert_eq!(q.pop().await.unwrap().entrypoint, "high-2");
    // sonra low'lar, kendi aralarında FIFO
    assert_eq!(q.pop().await.unwrap().entrypoint, "low-1");
    assert_eq!(q.pop().await.unwrap().entrypoint, "low-2");
}
