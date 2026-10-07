// ============================================================
// src/agents/executor.rs
//
// Haziran 2026: AgentRuntime::execute() artık ai_router parametresi
// de alıyor (kullanıcının Ayarlar'da aktif ettiği AI provider).
// AgentExecutor bu parametreyi alıp olduğu gibi iletir.
//
// V10 Sprint 5: dönüş tipi Uuid'den AgentOutcome'a değişti —
// execution artık ya Completed ile ya da PendingApproval ile
// bitebilir (bkz. agents::runtime::AgentOutcome).
//
// V10 Sprint 6: audit_log parametresi eklendi — Governor kararları,
// tool çağrıları ve pause/resume olayları artık gerçek bir denetim
// izine yazılabiliyor (bkz. logging::audit::AuditLog).
// ============================================================

use std::sync::Arc;

use crate::agents::approval::ApprovalStore;
use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::runtime::{AgentOutcome, AgentRuntime};
use crate::agents::tools::AgentTool;
use crate::ai::routing::router::ProviderRouter;
use crate::errors::runtime::RuntimeError;
use crate::logging::audit::AuditLog;
use crate::security::capability_engine::CapabilityEngine;
use crate::security::risk_engine::RiskEngine;

pub struct AgentExecutor;

impl AgentExecutor {
    pub async fn execute(
        context: AgentContext,
        objective: String,
        budget: AgentExecutionBudget,
        tools: Vec<Arc<dyn AgentTool>>,
        ai_router: Option<Arc<ProviderRouter>>,
        capability_engine: Option<Arc<CapabilityEngine>>,
        risk_engine: Option<Arc<RiskEngine>>,
        approval_store: Option<Arc<ApprovalStore>>,
        audit_log: Option<Arc<AuditLog>>,
    ) -> Result<AgentOutcome, RuntimeError> {
        AgentRuntime::execute(
            context,
            objective,
            budget,
            tools,
            ai_router,
            capability_engine,
            risk_engine,
            approval_store,
            audit_log,
        )
        .await
    }
}
