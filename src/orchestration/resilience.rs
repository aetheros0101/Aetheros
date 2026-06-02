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
pub struct ResiliencePolicy {
    pub max_retries:
        usize,

    pub replay_enabled:
        bool,

    pub failover_enabled:
        bool,

    pub checkpointing:
        bool,
}
