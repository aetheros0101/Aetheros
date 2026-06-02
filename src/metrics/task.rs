use std::sync::atomic::{
    AtomicU64,
    Ordering,
};

pub struct TaskMetrics {
    completed: AtomicU64,
    failed: AtomicU64,
    cancelled: AtomicU64,
    retries: AtomicU64,
}

impl TaskMetrics {
    pub fn new() -> Self {
        Self {
            completed:
                AtomicU64::new(0),

            failed:
                AtomicU64::new(0),

            cancelled:
                AtomicU64::new(0),

            retries:
                AtomicU64::new(0),
        }
    }

    pub fn increment_completed(
        &self,
    ) {
        self.completed.fetch_add(
            1,
            Ordering::Relaxed,
        );
    }

    pub fn increment_failed(
        &self,
    ) {
        self.failed.fetch_add(
            1,
            Ordering::Relaxed,
        );
    }

    pub fn increment_cancelled(
        &self,
    ) {
        self.cancelled.fetch_add(
            1,
            Ordering::Relaxed,
        );
    }

    pub fn increment_retries(
        &self,
    ) {
        self.retries.fetch_add(
            1,
            Ordering::Relaxed,
        );
    }
}
