use std::sync::Arc;

use tracing::info;

use crate::agents::approval::{ApprovalStore, PendingApproval};
use crate::agents::budget::{AgentExecutionBudget, BudgetAccounting};
use crate::agents::cancellation::CancellationToken;
use crate::agents::context::AgentContext;
use crate::agents::observability::{AgentEvent, AgentEventKind, AgentEventSink};
use crate::agents::planner::StepRecord;
use crate::agents::reasoning::{ReasoningPhase, ReasoningTrace};
use crate::agents::state::AgentState;
use crate::agents::tools::AgentTool;
use crate::ai::routing::router::ProviderRouter;
use crate::errors::runtime::RuntimeError;
use crate::logging::audit::{AuditEventKind, AuditLog, summarize_output};
use crate::security::capability_engine::CapabilityEngine;
use crate::security::governor::GovernorDecision;
use crate::security::risk_engine::RiskEngine;

use super::{AgentOutcome, AgentRuntime, now_ms};

impl AgentRuntime {
    /// Agent execution döngüsü.
    ///
    /// objective → plan → adım adım çalıştır → AgentOutcome
    ///
    /// `ai_router`: kullanıcının Ayarlar'da aktif ettiği AI provider'a
    /// erişim sağlar (None ise plan üretimi fallback_plan'a düşer —
    /// runtime hiç donmaz).
    #[allow(clippy::too_many_arguments)] // TODO(Faz 2): parametre struct'ı
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
        info!(
            agent_id = %context.agent_id,
            execution_id = %context.execution_id,
            objective = %objective,
            "Agent execution started"
        );

        let mut runtime = Self::new(
            budget,
            tools,
            ai_router,
            capability_engine,
            risk_engine,
            approval_store,
            audit_log,
        );
        // B10: üretim giriş noktası — eksik motor "izin ver" demek değildir.
        runtime.governor.set_fail_closed(true);
        runtime.begin_execution(&context, &objective);
        let result = runtime.run(context.clone(), objective).await;
        runtime.finish_execution(&context, &result);
        result
    }

    /// Cancel token + event sink ile execution (kurumsal hostlar).
    #[allow(clippy::too_many_arguments)] // TODO(Faz 2): parametre struct'ı
    pub async fn execute_with_control(
        context: AgentContext,
        objective: String,
        budget: AgentExecutionBudget,
        tools: Vec<Arc<dyn AgentTool>>,
        ai_router: Option<Arc<ProviderRouter>>,
        capability_engine: Option<Arc<CapabilityEngine>>,
        risk_engine: Option<Arc<RiskEngine>>,
        approval_store: Option<Arc<ApprovalStore>>,
        audit_log: Option<Arc<AuditLog>>,
        cancellation: CancellationToken,
        event_sink: Option<Arc<dyn AgentEventSink>>,
    ) -> Result<AgentOutcome, RuntimeError> {
        info!(
            agent_id = %context.agent_id,
            execution_id = %context.execution_id,
            objective = %objective,
            "Agent execution started (controlled)"
        );
        let mut runtime = Self::new_with_control(
            budget,
            tools,
            ai_router,
            capability_engine,
            risk_engine,
            approval_store,
            audit_log,
            cancellation,
            event_sink,
        );
        runtime.governor.set_fail_closed(true);
        runtime.begin_execution(&context, &objective);
        let result = runtime.run(context.clone(), objective).await;
        runtime.finish_execution(&context, &result);
        result
    }

    fn begin_execution(&mut self, context: &AgentContext, objective: &str) {
        self.accounting = BudgetAccounting::start(now_ms());
        self.emit(context, AgentEventKind::ExecutionStarted);
        self.transition(context, AgentState::Initializing, None);
        self.transition(
            context,
            AgentState::Planning,
            Some("objective received".into()),
        );
        self.reasoning_log.push(ReasoningTrace::new(
            context.agent_id.to_string(),
            ReasoningPhase::Plan,
            format!("objective: {objective}"),
        ));
        // AuditEventKind has no ExecutionStarted — host audit covers start via other events.
    }

    fn finish_execution(
        &mut self,
        context: &AgentContext,
        result: &Result<AgentOutcome, RuntimeError>,
    ) {
        match result {
            Ok(AgentOutcome::Completed) => {
                self.transition(context, AgentState::Completed, None);
                self.emit(context, AgentEventKind::ExecutionCompleted);
            }
            Ok(AgentOutcome::PendingApproval { approval_id }) => {
                self.transition(
                    context,
                    AgentState::Waiting,
                    Some(format!("approval {approval_id}")),
                );
                let mut ev = AgentEvent::new(
                    AgentEventKind::ApprovalRequested,
                    context.agent_id,
                    context.execution_id,
                );
                ev.message = Some(approval_id.to_string());
                self.emit_rich(ev);
            }
            Err(e) => {
                if self.cancellation.is_cancelled() {
                    self.lifecycle.force_terminal(
                        AgentState::Cancelled,
                        Some("cancelled".into()),
                        now_ms(),
                    );
                    self.emit(context, AgentEventKind::ExecutionCancelled);
                } else {
                    self.lifecycle.force_terminal(
                        AgentState::Failed,
                        Some(e.to_string()),
                        now_ms(),
                    );
                    self.emit(context, AgentEventKind::ExecutionFailed);
                }
            }
        }
    }

    /// V10 Sprint 5: onaylanmış bir PendingApproval'ı devam ettirir.
    ///
    /// Önce onay bekleyen tool_call'ı invoke eder — Governor'a TEKRAR
    /// sormadan, çünkü kullanıcı zaten açıkça onayladı (yeniden sorsak
    /// aynı RequiresApproval'ı alıp asla ilerleyemeyiz). Sonra
    /// `remaining_steps` ile normal döngüye devam eder.
    ///
    /// `tools`/`ai_router`/`capability_engine`/`risk_engine`/
    /// `approval_store`: resume anında çağıran tarafın (bridge) canlı
    /// runtime'ından TAZE olarak sağlanır — PendingApproval bunları
    /// saklamıyor (serialize edilemezler), sadece "kaldığı yer" bilgisini
    /// saklıyor.
    pub async fn resume(
        pending: PendingApproval,
        tools: Vec<Arc<dyn AgentTool>>,
        ai_router: Option<Arc<ProviderRouter>>,
        capability_engine: Option<Arc<CapabilityEngine>>,
        risk_engine: Option<Arc<RiskEngine>>,
        approval_store: Option<Arc<ApprovalStore>>,
        audit_log: Option<Arc<AuditLog>>,
    ) -> Result<AgentOutcome, RuntimeError> {
        info!(
            agent_id = %pending.context.agent_id,
            execution_id = %pending.context.execution_id,
            approval_id = %pending.id,
            "Resuming approved agent execution"
        );

        let mut runtime = Self::new(
            pending.budget.clone(),
            tools,
            ai_router,
            capability_engine,
            risk_engine,
            approval_store,
            audit_log,
        );

        // B10: üretim giriş noktası — eksik motor "izin ver" demek değildir.
        runtime.governor.set_fail_closed(true);
        // Resume: Waiting → Executing (onay sonrası)
        runtime.accounting = BudgetAccounting::start(now_ms());
        // Resume control plane: Registered → … → Waiting → Executing
        let _ = runtime
            .lifecycle
            .transition(AgentState::Initializing, None, now_ms());
        let _ = runtime
            .lifecycle
            .transition(AgentState::Planning, None, now_ms());
        let _ = runtime
            .lifecycle
            .transition(AgentState::Executing, None, now_ms());
        let _ = runtime.lifecycle.transition(
            AgentState::Waiting,
            Some("pending approval".into()),
            now_ms(),
        );
        runtime.transition(
            &pending.context,
            AgentState::Executing,
            Some(format!("approval {} resolved", pending.id)),
        );
        let mut ev = AgentEvent::new(
            AgentEventKind::ApprovalResolved,
            pending.context.agent_id,
            pending.context.execution_id,
        );
        ev.message = Some(pending.id.to_string());
        runtime.emit_rich(ev);

        runtime.audit(
            pending.context.agent_id,
            pending.context.execution_id,
            AuditEventKind::ExecutionResumed {
                approval_id: pending.id,
            },
        );

        // Onaylanan çağrıyı invoke et. Governor'ı TAMAMEN atlamıyoruz (B7):
        // kullanıcı onayı yalnızca "RequiresApproval"ı aşar; capability
        // reddi (grant sonradan geri alınmış olabilir) ve argüman
        // politikasının kesin reddi (Deny) onayla AŞILAMAZ.
        let tool = runtime
            .tools
            .iter()
            .find(|t| t.name() == pending.tool_call.tool_name)
            .cloned();

        let result = match tool {
            Some(tool) => match runtime.governor.evaluate_call(
                pending.context.agent_id,
                &tool,
                &pending.tool_call.arguments,
            ) {
                GovernorDecision::Deny { reason } => Err(format!(
                    "onaylanan çağrı artık reddediliyor (capability/politika): {reason}"
                )),
                // Allow veya RequiresApproval → kullanıcı zaten onayladı.
                _ => tool.invoke(pending.tool_call.arguments.clone()).await,
            },
            None => Err(format!(
                "onaylanan tool artık mevcut değil: '{}'",
                pending.tool_call.tool_name
            )),
        };

        let approved_output: String;
        match result {
            Ok(output) => {
                runtime.audit(
                    pending.context.agent_id,
                    pending.context.execution_id,
                    AuditEventKind::ToolInvoked {
                        tool_name: pending.tool_call.tool_name.clone(),
                        success: true,
                        error: None,
                        arguments: pending.tool_call.arguments.clone(),
                        output: Some(summarize_output(&output)),
                    },
                );
                approved_output = output.clone();
                runtime
                    .memory
                    .store(pending.id.to_string(), serde_json::Value::String(output));
            }
            Err(e) => {
                runtime.audit(
                    pending.context.agent_id,
                    pending.context.execution_id,
                    AuditEventKind::ToolInvoked {
                        tool_name: pending.tool_call.tool_name.clone(),
                        success: false,
                        error: Some(e.clone()),
                        arguments: pending.tool_call.arguments.clone(),
                        output: None,
                    },
                );
                let message = format!("Onaylanan tool call başarısız oldu: {e}");
                runtime.audit(
                    pending.context.agent_id,
                    pending.context.execution_id,
                    AuditEventKind::ExecutionFailed {
                        error: message.clone(),
                    },
                );
                return Err(RuntimeError::TaskExecutionFailed { message });
            }
        }

        // B6: duraklayan execution OTONOM döngüdeyse, onaylanan adım geçmişe
        // eklenir ve planlayıcıyla DEVAM edilir (eskiden onaylanan adımdan
        // sonra execution orada bitiyordu).
        if pending.autonomous
            && let Some(router) = runtime.ai_router.clone()
        {
            let mut history: Vec<StepRecord> =
                pending.history.iter().map(StepRecord::from).collect();
            let tokens = approved_output.split_whitespace().count();
            history.push(StepRecord {
                step_name: format!("{} (onaylandı)", pending.tool_call.tool_name),
                success: true,
                output: approved_output,
                tool_call: Some(pending.tool_call.clone()),
            });
            return runtime
                .run_autonomous_steps(
                    &pending.context,
                    &pending.objective,
                    router,
                    history,
                    1,
                    tokens,
                )
                .await;
        }

        runtime
            .run_steps(
                &pending.context,
                &pending.remaining_steps,
                0,
                0,
                &pending.objective,
            )
            .await
    }
}
