// ============================================================
// src/tests/dashboard_tests.rs
//
// SPRINT 8 — Dashboard + Metrics Testleri
// ============================================================

use std::sync::Arc;

use crate::events::bus::{EventBus, SystemEvent};
use crate::events::task::TaskEvent;
use crate::metrics::runtime::RuntimeMetrics;
use crate::types::ids::TaskId;
use uuid::Uuid;

fn task_id() -> TaskId { TaskId(Uuid::new_v4()) }

// ── RuntimeMetrics Unit Testleri ──────────────────────────

#[test]
fn metrics_initial_values_zero() {
    let m = RuntimeMetrics::new();
    let s = m.snapshot();
    assert_eq!(s.completed_tasks, 0);
    assert_eq!(s.failed_tasks, 0);
    assert_eq!(s.queued_tasks, 0);
    assert_eq!(s.retried_tasks, 0);
    assert_eq!(s.active_workers, 0);
}

#[test]
fn metrics_increment_completed() {
    let m = RuntimeMetrics::new();
    m.increment_queued();
    m.increment_completed();
    let s = m.snapshot();
    assert_eq!(s.completed_tasks, 1);
    assert_eq!(s.queued_tasks, 0); // queued azaldı
}

#[test]
fn metrics_increment_failed() {
    let m = RuntimeMetrics::new();
    m.increment_queued();
    m.increment_failed();
    let s = m.snapshot();
    assert_eq!(s.failed_tasks, 1);
    assert_eq!(s.queued_tasks, 0);
}

#[test]
fn metrics_increment_retried() {
    let m = RuntimeMetrics::new();
    m.increment_retried();
    assert_eq!(m.snapshot().retried_tasks, 1);
}

#[test]
fn metrics_queued_never_goes_negative() {
    let m = RuntimeMetrics::new();
    // Queued olmadan completed çağrılırsa
    m.increment_completed(); // saturating_sub → 0
    assert_eq!(m.snapshot().queued_tasks, 0);
}

#[test]
fn metrics_set_active_workers() {
    let m = RuntimeMetrics::new();
    m.set_active_workers(4);
    assert_eq!(m.snapshot().active_workers, 4);
    m.set_active_workers(2);
    assert_eq!(m.snapshot().active_workers, 2);
}

#[test]
fn metrics_multiple_operations() {
    let m = RuntimeMetrics::new();

    for _ in 0..10 { m.increment_queued(); }
    for _ in 0..7  { m.increment_completed(); }
    for _ in 0..2  { m.increment_failed(); }
    m.increment_retried();

    let s = m.snapshot();
    assert_eq!(s.queued_tasks, 1);     // 10 - 7 - 2 = 1
    assert_eq!(s.completed_tasks, 7);
    assert_eq!(s.failed_tasks, 2);
    assert_eq!(s.retried_tasks, 1);
}

// ── EventBus → Metrics Entegrasyon Testleri ───────────────

#[tokio::test]
async fn metrics_collector_processes_task_completed() {
    let bus = EventBus::new(32);
    let metrics = Arc::new(RuntimeMetrics::new());

    metrics.increment_queued(); // önce kuyruğa al

    metrics.clone().start_collecting(bus.clone());

    // Kısa bekleme — collector spawn'landı
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    bus.publish(SystemEvent::Task(
        TaskEvent::TaskCompleted { task_id: task_id() }
    ));

    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

    let s = metrics.snapshot();
    assert_eq!(s.completed_tasks, 1);
}

#[tokio::test]
async fn metrics_collector_processes_task_failed() {
    let bus = EventBus::new(32);
    let metrics = Arc::new(RuntimeMetrics::new());
    metrics.increment_queued();
    metrics.clone().start_collecting(bus.clone());

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    bus.publish(SystemEvent::Task(
        TaskEvent::TaskFailed { task_id: task_id() }
    ));

    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    assert_eq!(metrics.snapshot().failed_tasks, 1);
}

#[tokio::test]
async fn metrics_collector_processes_task_queued() {
    let bus = EventBus::new(32);
    let metrics = Arc::new(RuntimeMetrics::new());
    metrics.clone().start_collecting(bus.clone());

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    bus.publish(SystemEvent::Task(
        TaskEvent::TaskQueued { task_id: task_id() }
    ));

    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    assert_eq!(metrics.snapshot().queued_tasks, 1);
}

#[tokio::test]
async fn metrics_collector_processes_retried() {
    let bus = EventBus::new(32);
    let metrics = Arc::new(RuntimeMetrics::new());
    metrics.clone().start_collecting(bus.clone());

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    bus.publish(SystemEvent::Task(
        TaskEvent::TaskRetried { task_id: task_id(), attempt: 2 }
    ));

    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    assert_eq!(metrics.snapshot().retried_tasks, 1);
}

#[tokio::test]
async fn metrics_collector_multiple_events() {
    let bus = EventBus::new(64);
    let metrics = Arc::new(RuntimeMetrics::new());
    metrics.clone().start_collecting(bus.clone());

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // 5 queued, 3 completed, 1 failed, 1 retried
    for _ in 0..5 {
        bus.publish(SystemEvent::Task(
            TaskEvent::TaskQueued { task_id: task_id() }
        ));
    }
    for _ in 0..3 {
        bus.publish(SystemEvent::Task(
            TaskEvent::TaskCompleted { task_id: task_id() }
        ));
    }
    bus.publish(SystemEvent::Task(
        TaskEvent::TaskFailed { task_id: task_id() }
    ));
    bus.publish(SystemEvent::Task(
        TaskEvent::TaskRetried { task_id: task_id(), attempt: 1 }
    ));

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let s = metrics.snapshot();
    assert_eq!(s.completed_tasks, 3);
    assert_eq!(s.failed_tasks, 1);
    assert_eq!(s.retried_tasks, 1);
    // queued: 5 eklendi, 3+1=4 azaldı → 1
    assert_eq!(s.queued_tasks, 1);
}

// ── Dashboard HTML Testleri ───────────────────────────────

#[test]
fn dashboard_html_contains_key_elements() {
    use crate::api::dashboard::DASHBOARD_HTML;

    assert!(DASHBOARD_HTML.contains("AetherOS"));
    assert!(DASHBOARD_HTML.contains("/ws"));
    assert!(DASHBOARD_HTML.contains("m-total"));
}

#[test]
fn dashboard_html_has_metric_ids() {
    use crate::api::dashboard::DASHBOARD_HTML;

    // JS tarafında güncellenen ID'ler
    assert!(DASHBOARD_HTML.contains("m-total"));
    assert!(DASHBOARD_HTML.contains("m-active"));
    assert!(DASHBOARD_HTML.contains("m-completed"));
    assert!(DASHBOARD_HTML.contains("m-failed"));
    assert!(DASHBOARD_HTML.contains("m-retried"));
}

#[test]
fn dashboard_html_has_reconnect_logic() {
    use crate::api::dashboard::DASHBOARD_HTML;
    assert!(DASHBOARD_HTML.contains("reconnect"));
    assert!(DASHBOARD_HTML.contains("setTimeout"));
}
