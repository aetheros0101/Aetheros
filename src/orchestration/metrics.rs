use std::sync::atomic::{
    AtomicU64,
    Ordering,
};

pub struct ExecutionMetrics {
    active_executions:
        AtomicU64,

    completed_executions:
        AtomicU64,

    failed_executions:
        AtomicU64,
}

impl ExecutionMetrics {
    pub fn new() -> Self {
        Self {
            active_executions:
                AtomicU64::new(0),

            completed_executions:
                AtomicU64::new(0),

            failed_executions:
                AtomicU64::new(0),
        }
    }

    pub fn increment_active(
        &self,
    ) {
        self.active_executions
            .fetch_add(
                1,
                Ordering::SeqCst,
            );
    }

    pub fn increment_completed(
        &self,
    ) {
        self.completed_executions
            .fetch_add(
                1,
                Ordering::SeqCst,
            );
    }

    pub fn increment_failed(
        &self,
    ) {
        self.failed_executions
            .fetch_add(
                1,
                Ordering::SeqCst,
            );
    }
}
