use std::sync::atomic::{
    AtomicU64,
    Ordering,
};

pub struct MetricsAggregator {
    total_workflows:
        AtomicU64,

    failed_workflows:
        AtomicU64,
}

impl MetricsAggregator {
    pub fn new() -> Self {
        Self {
            total_workflows:
                AtomicU64::new(0),

            failed_workflows:
                AtomicU64::new(0),
        }
    }

    pub fn increment_total(
        &self,
    ) {
        self.total_workflows
            .fetch_add(
                1,
                Ordering::SeqCst,
            );
    }

    pub fn increment_failed(
        &self,
    ) {
        self.failed_workflows
            .fetch_add(
                1,
                Ordering::SeqCst,
            );
    }
}
