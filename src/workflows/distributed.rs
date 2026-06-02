use serde::{
    Deserialize,
    Serialize,
};

use uuid::Uuid;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct DistributedWorkflow {
    pub workflow_id:
        Uuid,

    pub assigned_nodes:
        Vec<Uuid>,
}
