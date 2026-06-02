use chrono::{
    DateTime,
    Utc,
};

use serde::{
    Deserialize,
    Serialize,
};

use crate::orchestration::state::OrchestrationState;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct RuntimeTrace {
    pub execution_id:
        String,

    pub state:
        OrchestrationState,

    pub node_id:
        Option<String>,

    pub timestamp:
        DateTime<Utc>,
}
