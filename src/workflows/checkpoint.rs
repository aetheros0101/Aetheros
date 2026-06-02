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
pub struct WorkflowCheckpoint {
    pub workflow_id: Uuid,

    pub state:
        WorkflowState,

    pub completed_nodes:
        Vec<Uuid>,

    pub pending_nodes:
        Vec<Uuid>,

    pub failed_nodes:
        Vec<Uuid>,
        
    pub retry_count:
        usize,    

    pub timestamp:
        DateTime<Utc>,
}
