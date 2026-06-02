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
pub struct StreamChunk {
    pub content: String,

    pub finished: bool,
}
