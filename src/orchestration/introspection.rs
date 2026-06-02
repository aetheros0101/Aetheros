use chrono::{
    DateTime,
    Utc,
};

use serde::{
    Deserialize,
    Serialize,
};

use uuid::Uuid;

use crate::orchestration::state::OrchestrationState;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct RuntimeSnapshot {
    pub execution_id:
        Uuid,

    pub state:
        OrchestrationState,

    pub active_nodes:
        usize,

    pub queued_nodes:
        usize,

    pub failed_nodes:
        usize,

    pub timestamp:
        DateTime<Utc>,
}
