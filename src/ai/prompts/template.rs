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
pub struct PromptTemplate {
    pub template_id:
        String,

    pub version:
        String,

    pub content:
        String,
}
