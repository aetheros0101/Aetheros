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
pub struct PromptVersion {
    pub id: String,

    pub version:
        String,

    pub checksum:
        String,
}
