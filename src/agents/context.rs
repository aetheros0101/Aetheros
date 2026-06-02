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
pub struct AgentContext {
    pub agent_id:
        Uuid,

    pub execution_id:
        Uuid,

    pub workflow_id:
        Option<Uuid>,
}
