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
pub struct ContextWindow {
    pub max_messages:
        usize,

    pub max_tokens:
        usize,
}
