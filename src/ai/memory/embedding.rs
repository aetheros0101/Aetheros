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
pub struct EmbeddingVector {
    pub values:
        Vec<f32>,
}
