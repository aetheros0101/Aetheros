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
pub struct AgentPlan {
    pub id: Uuid,

    pub objective:
        String,

    pub planned_steps:
        Vec<AgentPlanStep>,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct AgentPlanStep {
    pub id: Uuid,

    pub name: String,

    pub retryable: bool,
}
