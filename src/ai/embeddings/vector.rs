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
    pub dimensions:
        usize,

    pub values:
        Vec<f32>,
}
