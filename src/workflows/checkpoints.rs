use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};

use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCheckpoint {
    pub workflow_id: Uuid,

    pub node_id: Uuid,

    pub checkpoint_version: u64,

    pub created_at: DateTime<Utc>,
}
