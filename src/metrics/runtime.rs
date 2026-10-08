// ============================================================
// src/metrics/runtime.rs
//
// Faz 8: RuntimeMetrics EventBus'a bağlandı.
//
// ÖNCE: Alanlar tanımlı ama hiç güncellenmiyordu.
//       dead_code warning.
//
// SONRA:
//   - start_collecting() → EventBus'a subscribe olur
//   - TaskCompleted → increment_completed()
//   - TaskFailed    → increment_failed()
//   - TaskQueued    → increment_queued()
//   - snapshot()    → API veya log için anlık durum
//
// AtomicU64 + Relaxed: metrics için sıralama garantisi
// gerekmez, maksimum throughput önemli.
// ============================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tracing::debug;

use crate::events::bus::{EventBus, SystemEvent};
use crate::events::task::TaskEvent;

#[derive(Debug)]
pub struct MetricsSnapshot {
    pub active_workers: u64,
    pub queued_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub retried_tasks: u64,
}

pub struct RuntimeMetrics {
    active_workers: AtomicU64,
    queued_tasks: AtomicU64,
    completed_tasks: AtomicU64,
    failed_tasks: AtomicU64,
    retried_tasks: AtomicU64,
}

impl Default for RuntimeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeMetrics {
    pub fn new() -> Self {
        Self {
            active_workers: AtomicU64::new(0),
            queued_tasks: AtomicU64::new(0),
            completed_tasks: AtomicU64::new(0),
            failed_tasks: AtomicU64::new(0),
            retried_tasks: AtomicU64::new(0),
        }
    }

    // ── Manuel güncelleme API'si ──────────────────────────

    pub fn increment_completed(&self) {
        self.completed_tasks.fetch_add(1, Ordering::Relaxed);
        // Sıfırın altına düşme koruması
        self.queued_tasks
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                Some(v.saturating_sub(1))
            })
            .ok();
    }

    pub fn increment_failed(&self) {
        self.failed_tasks.fetch_add(1, Ordering::Relaxed);
        self.queued_tasks
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                Some(v.saturating_sub(1))
            })
            .ok();
    }

    pub fn increment_queued(&self) {
        self.queued_tasks.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_retried(&self) {
        self.retried_tasks.fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_active_workers(&self, count: u64) {
        self.active_workers.store(count, Ordering::Relaxed);
    }

    // ── Snapshot ──────────────────────────────────────────

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            active_workers: self.active_workers.load(Ordering::Relaxed),
            queued_tasks: self.queued_tasks.load(Ordering::Relaxed),
            completed_tasks: self.completed_tasks.load(Ordering::Relaxed),
            failed_tasks: self.failed_tasks.load(Ordering::Relaxed),
            retried_tasks: self.retried_tasks.load(Ordering::Relaxed),
        }
    }

    // ── EventBus Consumer ─────────────────────────────────

    /// EventBus'tan beslenme döngüsünü arka planda başlat.
    ///
    /// Her SystemEvent::Task event'ine göre metrikleri günceller.
    /// Shutdown: bus kapanınca (RecvError::Closed) döngü çıkar.
    pub fn start_collecting(self: Arc<Self>, bus: EventBus) {
        let mut receiver = bus.subscribe();

        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        self.process_event(&event);
                    }

                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        // Metrik toplama lag'ı — kayıp var ama runtime durmaz
                        debug!(skipped = n, "Metrics collector lagged");
                    }

                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        debug!("EventBus closed, metrics collector stopping");
                        break;
                    }
                }
            }
        });
    }

    pub fn process_event(&self, event: &SystemEvent) {
        if let SystemEvent::Task(task_event) = event {
            match task_event {
                TaskEvent::TaskQueued { .. } => {
                    self.increment_queued();
                }
                TaskEvent::TaskCompleted { .. } => {
                    self.increment_completed();
                }
                TaskEvent::TaskFailed { .. } => {
                    self.increment_failed();
                }
                TaskEvent::TaskRetried { .. } => {
                    self.increment_retried();
                }
                // TaskStarted, TaskCancelled → metrik yok şu an
                _ => {}
            }
        }
    }
}
