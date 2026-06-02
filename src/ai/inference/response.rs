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
pub struct InferenceResponse {
    pub output: String,

    pub tokens_used:
        usize,
}
