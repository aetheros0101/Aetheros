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
pub struct ToolInvocation {
    pub tool_name:
        String,

    pub correlation_id:
        String,

    pub arguments:
        Vec<String>,
}
