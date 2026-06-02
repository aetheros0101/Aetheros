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
pub struct OrchestrationEvent {
    pub orchestration_id:
        Uuid,

    pub execution_node_id:
        Option<Uuid>,

    pub correlation_id:
        String,

    pub parent_execution_id:
        Option<Uuid>,

    pub state:
        OrchestrationState,

    pub message:
        Option<String>,

    pub sequence_id:
            u64,    

    pub timestamp:
        DateTime<Utc>,
}
