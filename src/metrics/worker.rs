#[derive(Debug, Default)]
pub struct WorkerMetrics {
    pub active: bool,
    pub processed_tasks: u64,
}
