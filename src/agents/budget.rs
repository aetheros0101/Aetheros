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
pub struct AgentExecutionBudget {
    pub max_tokens:
        usize,

    pub max_steps:
        usize,

    pub max_runtime_seconds:
        usize,
}
