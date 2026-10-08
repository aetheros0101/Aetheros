use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TelemetryEvent {
    RuntimeMetric {
        name: String,
        value: f64,
    },

    TaskMetric {
        task_id: String,
        metric: String,
        value: f64,
    },

    WorkerMetric {
        worker_id: String,
        metric: String,
        value: f64,
    },

    PersistenceMetric {
        operation: String,
        latency_ms: u64,
    },
}
