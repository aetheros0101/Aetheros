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
pub struct RetrievalResult {
    pub content:
        String,

    pub score: f32,
}
