use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentExecutionState {
    pub active_tasks: usize,
    pub last_execution: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_outcome: Option<String>,
    #[serde(default)]
    pub consecutive_failures: u32,
}
