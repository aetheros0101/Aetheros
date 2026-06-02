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
pub struct ToolCall {
    pub tool_name:
        String,

    pub payload:
        String,
}
