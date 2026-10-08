use std::sync::atomic::{AtomicU64, Ordering};

pub struct RuntimeMetrics {
    completed_tasks: AtomicU64,
}

impl Default for RuntimeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeMetrics {
    pub fn new() -> Self {
        Self {
            completed_tasks: AtomicU64::new(0),
        }
    }

    pub fn increment_tasks(&self) {
        self.completed_tasks.fetch_add(1, Ordering::SeqCst);
    }

    pub fn completed_tasks(&self) -> u64 {
        self.completed_tasks.load(Ordering::SeqCst)
    }
}
