use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct IsolationPolicy {
    pub max_memory_mb:
        usize,

    pub max_cpu_time_ms:
        usize,

    pub allow_network:
        bool,
}
