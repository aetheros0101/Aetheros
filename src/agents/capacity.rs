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
pub struct AgentCapacity {
    pub max_concurrent_tasks:
        usize,

    pub active_tasks:
        usize,
}
