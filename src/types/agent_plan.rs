//src/types/agent_plan.rs
use serde::{
    Deserialize,
    Serialize,
};

use uuid::Uuid;

/// `agents` ve `ai` modüllerinin her ikisi de bu tipe ihtiyaç duyduğu için
/// döngüsel bağımlılığı önlemek amacıyla buraya (types) taşındı.
/// Önceki konum: src/agents/plans.rs
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
