use chrono::{
    DateTime,
    Utc,
};

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
pub struct AiInferenceTrace {
    pub provider: String,

    pub model: String,

    pub tokens_used:
        usize,

    pub latency_ms:
        u64,
            
    pub timestamp:
        DateTime<Utc>,
}
