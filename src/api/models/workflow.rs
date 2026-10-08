use serde::{Deserialize, Serialize};

use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSubmission {
    pub workflow_id: Uuid,

    pub name: String,
}
