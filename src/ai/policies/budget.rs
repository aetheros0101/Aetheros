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
pub struct TokenBudgetPolicy {
    pub max_input_tokens:
        usize,

    pub max_cost_usd:
        f32,
}
