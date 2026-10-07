use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub tool_name: String,
    pub correlation_id: String,
    pub arguments: Vec<String>,
    #[serde(default)]
    pub execution_id: Option<Uuid>,
    #[serde(default)]
    pub step_index: Option<u32>,
    #[serde(default)]
    pub started_at_ms: Option<u64>,
}

impl ToolInvocation {
    pub fn new(tool_name: impl Into<String>, arguments: Vec<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            correlation_id: Uuid::new_v4().to_string(),
            arguments,
            execution_id: None,
            step_index: None,
            started_at_ms: None,
        }
    }
}
