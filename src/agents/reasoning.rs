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
pub struct ReasoningTrace {
    pub agent_id:
        String,

    pub decision:
        String,

    pub timestamp:
        DateTime<Utc>,
}
