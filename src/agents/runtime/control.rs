use std::sync::Arc;

use tracing::warn;
use uuid::Uuid;

use crate::agents::budget::BudgetAccounting;
use crate::agents::cancellation::{CancellationMode, CancellationToken};
use crate::agents::context::AgentContext;
use crate::agents::observability::{AgentEvent, AgentEventKind};
use crate::agents::reasoning::ReasoningLog;
use crate::agents::state::AgentState;
use crate::agents::tools::AgentTool;
use crate::errors::runtime::RuntimeError;
use crate::logging::audit::AuditEventKind;

use super::{now_ms, AgentRuntime, HighRiskPolicy};

impl AgentRuntime {

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn lifecycle_state(&self) -> AgentState {
        self.lifecycle.state()
    }

    pub fn accounting(&self) -> &BudgetAccounting {
        &self.accounting
    }

    pub fn reasoning_log(&self) -> &ReasoningLog {
        &self.reasoning_log
    }

    pub(super) fn emit(&self, context: &AgentContext, kind: AgentEventKind) {
        if let Some(sink) = &self.event_sink {
            sink.emit(AgentEvent::new(kind, context.agent_id, context.execution_id));
        }
    }

    pub(super) fn emit_rich(&self, event: AgentEvent) {
        if let Some(sink) = &self.event_sink {
            sink.emit(event);
        }
    }

    pub(super) fn transition(
        &mut self,
        context: &AgentContext,
        to: AgentState,
        reason: Option<String>,
    ) {
        let now = now_ms();
        match self.lifecycle.transition(to, reason.clone(), now) {
            Ok(tr) => {
                let mut ev = AgentEvent::new(
                    AgentEventKind::StateTransition,
                    context.agent_id,
                    context.execution_id,
                );
                ev.message = Some(format!("{} → {}", tr.from.label(), tr.to.label()));
                if let Some(r) = reason {
                    ev.attributes.insert("reason".into(), serde_json::Value::String(r));
                }
                self.emit_rich(ev);
            }
            Err(e) => {
                warn!(
                    agent_id = %context.agent_id,
                    error = %e,
                    "lifecycle transition rejected"
                );
            }
        }
    }

    /// İptal bayrağı veya immediate mode — adım döngüsü başında.
    pub(super) fn check_cancelled(&self, context: &AgentContext) -> Result<(), RuntimeError> {
        if !self.cancellation.is_cancelled() {
            return Ok(());
        }
        let mode = self.cancellation.mode().unwrap_or(CancellationMode::Graceful);
        self.emit(context, AgentEventKind::CancellationRequested);
        self.audit(
            context.agent_id,
            context.execution_id,
            AuditEventKind::ExecutionFailed {
                error: format!("cancelled ({mode:?})"),
            },
        );
        Err(RuntimeError::TaskExecutionFailed {
            message: format!("agent cancelled ({mode:?})"),
        })
    }

    pub(super) fn check_budget_accounting(&self, context: &AgentContext) -> Result<(), RuntimeError> {
        if let Err(e) = self.accounting.check(&self.budget, now_ms()) {
            self.emit(context, AgentEventKind::BudgetExceeded);
            self.audit(
                context.agent_id,
                context.execution_id,
                AuditEventKind::ExecutionFailed {
                    error: e.message.clone(),
                },
            );
            return Err(RuntimeError::TaskExecutionFailed {
                message: e.message,
            });
        }
        // %80 uyarı
        let steps_ratio = self.accounting.steps_used as f32
            / self.budget.max_steps.max(1) as f32;
        if steps_ratio >= 0.8 {
            self.emit(context, AgentEventKind::BudgetWarning);
        }
        Ok(())
    }

    /// Denetim izine tek bir olay yaz. audit_log bağlanmamışsa (None)
    /// sessizce hiçbir şey yapmaz.
    pub(super) fn audit(&self, agent_id: Uuid, execution_id: Uuid, kind: AuditEventKind) {
        if let Some(log) = &self.audit_log {
            log.record(agent_id, execution_id, kind);
        }
    }

    /// Security Governor'ın high-risk politikasını değiştirir
    /// (varsayılan RequireApproval — High riskli çağrı onay bekler). bkz. security::governor::HighRiskPolicy.
    #[allow(dead_code)]
    pub(crate) fn set_high_risk_policy(&mut self, policy: HighRiskPolicy) {
        self.governor.set_high_risk_policy(policy);
    }

    /// Agent execution döngüsü.
    ///
    /// objective → plan → adım adım çalıştır → AgentOutcome
    ///
    /// `ai_router`: kullanıcının Ayarlar'da aktif ettiği AI provider'a
    /// erişim sağlar (None ise plan üretimi fallback_plan'a düşer —
    /// aksi hâlde model olmayan yetkiyle araç seçip aynı reddi tekrar tekrar alıyordu.
    pub(super) fn usable_tools(&self, agent_id: Uuid) -> Vec<Arc<dyn AgentTool>> {
        match &self.capability_engine {
            None => self.tools.clone(),
            Some(engine) => self
                .tools
                .iter()
                .filter(|t| engine.check(&agent_id, t.required_capability()).is_allowed())
                .cloned()
                .collect(),
        }
    }

    /// Execution'ı denetime `ExecutionFailed` yazarak başarısız kapatır.
    pub(super) fn fail_execution(&self, context: &AgentContext, message: String) -> RuntimeError {
        self.emit(context, AgentEventKind::ExecutionFailed);
        self.audit(
            context.agent_id,
            context.execution_id,
            AuditEventKind::ExecutionFailed { error: message.clone() },
        );
        RuntimeError::TaskExecutionFailed { message }
    }

}
