use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub worker_count: usize,

    pub task_channel_capacity: usize,

    pub event_channel_capacity: usize,

    pub shutdown_timeout: Duration,

    pub max_concurrent_tasks: usize,

    pub persistence_path: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_count: 4,

            task_channel_capacity: 1024,

            event_channel_capacity: 2048,

            shutdown_timeout: Duration::from_secs(30),

            max_concurrent_tasks: 1024,

            persistence_path: "./aetheros.db".into(),
        }
    }
}
