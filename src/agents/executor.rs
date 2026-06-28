// ============================================================
// src/agents/executor.rs
//
// Haziran 2026: AgentRuntime::execute() artık ai_router parametresi
// de alıyor (kullanıcının Ayarlar'da aktif ettiği AI provider).
// AgentExecutor bu parametreyi alıp olduğu gibi iletir.
// ============================================================

use std::sync::Arc;

use uuid::Uuid;

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::runtime::AgentRuntime;
use crate::agents::tools::AgentTool;
use crate::ai::routing::router::ProviderRouter;
use crate::errors::runtime::RuntimeError;

pub struct AgentExecutor;

impl AgentExecutor {
    pub async fn execute(
        context: AgentContext,
        objective: String,
        budget: AgentExecutionBudget,
        tools: Vec<Arc<dyn AgentTool>>,
        ai_router: Option<Arc<ProviderRouter>>,
    ) -> Result<Uuid, RuntimeError> {
        AgentRuntime::execute(
            context,
            objective,
            budget,
            tools,
            ai_router,
        )
        .await
    }
}
