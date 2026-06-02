use chrono::{
    DateTime,
    Utc,
};

use serde::{
    Deserialize,
    Serialize,
};

use uuid::Uuid;

use crate::workflows::state::WorkflowState;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct WorkflowEvent {
    pub workflow_id: Uuid,

    pub node_id:
        Option<Uuid>,

    pub state:
        WorkflowState,

    pub timestamp:
        DateTime<Utc>,
}
