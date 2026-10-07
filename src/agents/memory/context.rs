//! Execution context — host ile birebir uyumlu (3 alan).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub agent_id: Uuid,
    pub execution_id: Uuid,
    pub workflow_id: Option<Uuid>,
}

impl AgentContext {
    pub fn new(agent_id: Uuid) -> Self {
        Self {
            agent_id,
            execution_id: Uuid::new_v4(),
            workflow_id: None,
        }
    }

    pub fn correlation_id(&self) -> String {
        format!("{}:{}", self.agent_id, self.execution_id)
    }
}
