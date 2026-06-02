use chrono::{
    DateTime,
    Utc,
};

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
pub struct AgentExecutionState {
    pub active_tasks:
        usize,

    pub last_execution:
        Option<DateTime<Utc>>,
}
