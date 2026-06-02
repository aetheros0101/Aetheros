pub mod plan;

use async_trait::async_trait;

use crate::ai::errors::AiError;

use crate::agents::plans::AgentPlan;

use uuid::Uuid;

#[async_trait]
pub trait AiPlanner:
    Send + Sync
{
    async fn create_plan(
        &self,
        objective: String,
    ) -> Result<
        AgentPlan,
        AiError,
    >;
}

#[derive(
    Debug,
    Clone,
)]
pub struct AiExecutionPlan {
    pub execution_id:
        Uuid,

    pub reasoning_steps:
        Vec<String>,
}
