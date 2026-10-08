// ============================================================
// src/tests/integration/eventbus_tests.rs
//
// SPRINT 2 — EventBus Entegrasyon Testleri
//
// Test edilen özellikler:
//   - publish → subscribe round-trip
//   - Birden fazla subscriber aynı event'i alır
//   - Task event'leri doğru variant'ta iletilir
//   - Runtime event'leri doğru iletilir
//   - Buffer dolu olunca Lagged hatası (panic değil)
//   - Subscriber yokken publish → panic yok
// ============================================================

use tokio::time::{Duration, timeout};

use crate::events::bus::{EventBus, SystemEvent};
use crate::events::runtime::RuntimeEvent;
use crate::events::task::TaskEvent;
use crate::types::ids::TaskId;
use uuid::Uuid;

// ── Yardımcılar ───────────────────────────────────────────

fn task_id() -> TaskId {
    TaskId(Uuid::new_v4())
}

// ── Temel Testler ─────────────────────────────────────────

/// publish → subscribe: event alınmalı.
#[tokio::test]
async fn publish_and_receive_event() {
    let bus = EventBus::new(16);
    let mut rx = bus.subscribe();

    let id = task_id();
    bus.publish(SystemEvent::Task(TaskEvent::TaskCompleted { task_id: id }));

    let event = timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("timeout")
        .expect("bus closed");

    match event {
        SystemEvent::Task(TaskEvent::TaskCompleted { task_id }) => {
            assert_eq!(task_id, id);
        }
        other => panic!("Beklenmedik event: {:?}", other),
    }
}

/// Birden fazla subscriber aynı event'i almalı.
#[tokio::test]
async fn multiple_subscribers_receive_same_event() {
    let bus = EventBus::new(16);
    let mut rx1 = bus.subscribe();
    let mut rx2 = bus.subscribe();
    let mut rx3 = bus.subscribe();

    let id = task_id();
    bus.publish(SystemEvent::Task(TaskEvent::TaskStarted { task_id: id }));

    for rx in [&mut rx1, &mut rx2, &mut rx3] {
        let event = timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("timeout")
            .expect("bus closed");

        match event {
            SystemEvent::Task(TaskEvent::TaskStarted { task_id }) => assert_eq!(task_id, id),
            _ => panic!("Beklenmedik event"),
        }
    }
}

/// Runtime event'leri doğru iletilmeli.
#[tokio::test]
async fn runtime_events_delivered() {
    let bus = EventBus::new(16);
    let mut rx = bus.subscribe();

    bus.publish(SystemEvent::Runtime(RuntimeEvent::RuntimeStarted));

    let event = timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("timeout")
        .expect("bus closed");

    assert!(matches!(
        event,
        SystemEvent::Runtime(RuntimeEvent::RuntimeStarted)
    ));
}

/// Subscriber yokken publish → panic olmamalı.
#[tokio::test]
async fn publish_without_subscriber_no_panic() {
    let bus = EventBus::new(16);
    // Subscriber oluşturulmadı

    // Bu panic etmemeli — broadcast::Sender::send() err döner,
    // EventBus::publish() ise hatayı yoksayar.
    bus.publish(SystemEvent::Task(TaskEvent::TaskFailed {
        task_id: task_id(),
    }));
    bus.publish(SystemEvent::Runtime(RuntimeEvent::RuntimeStopped));
    // Buraya ulaştıysak test geçti
}

/// Sıralı publish → sıralı receive (FIFO).
#[tokio::test]
async fn events_received_in_order() {
    let bus = EventBus::new(32);
    let mut rx = bus.subscribe();

    let ids: Vec<TaskId> = (0..5).map(|_| task_id()).collect();

    for id in &ids {
        bus.publish(SystemEvent::Task(TaskEvent::TaskQueued { task_id: *id }));
    }

    for expected_id in &ids {
        let event = timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("timeout")
            .expect("closed");

        match event {
            SystemEvent::Task(TaskEvent::TaskQueued { task_id }) => {
                assert_eq!(&task_id, expected_id)
            }
            _ => panic!("Beklenmedik event"),
        }
    }
}

/// TaskRetried event'i attempt bilgisiyle iletilmeli.
#[tokio::test]
async fn task_retried_event_carries_attempt_info() {
    let bus = EventBus::new(16);
    let mut rx = bus.subscribe();

    let id = task_id();
    bus.publish(SystemEvent::Task(TaskEvent::TaskRetried {
        task_id: id,
        attempt: 2,
    }));

    let event = timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("timeout")
        .unwrap();

    match event {
        SystemEvent::Task(TaskEvent::TaskRetried { task_id, attempt }) => {
            assert_eq!(task_id, id);
            assert_eq!(attempt, 2);
        }
        _ => panic!("TaskRetried bekliyordu"),
    }
}

/// Bus kapasitesi küçükse Lagged hatası — panic değil.
#[tokio::test]
async fn lagged_subscriber_gets_lagged_error() {
    use tokio::sync::broadcast::error::RecvError;

    // Kapasite = 2, 10 event publish
    let bus = EventBus::new(2);
    let mut rx = bus.subscribe();

    for _ in 0..10 {
        bus.publish(SystemEvent::Task(TaskEvent::TaskCompleted {
            task_id: task_id(),
        }));
    }

    // İlk recv() ya Ok ya Lagged dönmeli — hiçbiri panic etmez
    let result = rx.recv().await;
    match result {
        Ok(_) => {}                     // Buffer'da yer varsa
        Err(RecvError::Lagged(_)) => {} // Buffer doluysa beklenen
        Err(RecvError::Closed) => {
            panic!("Bus beklenmedik şekilde kapandı")
        }
    }
}
