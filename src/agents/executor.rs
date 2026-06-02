// ============================================================
// src/agents/executor.rs
//
// Faz 5 Güncelleme:
//   AgentRuntime::execute() imzası değişti (4 argüman).
//   AgentExecutor bu argümanları alıp iletir.
//   Dönüş tipi Uuid → Result<Uuid, RuntimeError>.
// ============================================================

use std::sync::Arc;

use uuid::Uuid;

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::runtime::AgentRuntime;
use crate::agents::tools::AgentTool;
use crate::errors::runtime::RuntimeError;

pub struct AgentExecutor;

impl AgentExecutor {
    pub async fn execute(
        context: AgentContext,
        objective: String,
        budget: AgentExecutionBudget,
        tools: Vec<Arc<dyn AgentTool>>,
    ) -> Result<Uuid, RuntimeError> {
        AgentRuntime::execute(
            context,
            objective,
            budget,
            tools,
        )
        .await
    }
}
