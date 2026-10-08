use std::sync::Arc;

use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::agents::approval::{PendingApproval, StepSnapshot};
use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::planner::StepRecord;
use crate::agents::plans::{AgentPlanStep, ToolCall};
use crate::agents::reasoning::ReasoningTrace;
use crate::agents::tools::AgentTool;
use crate::errors::runtime::RuntimeError;
use crate::logging::audit::{AuditEventKind, summarize_output};
use crate::security::governor::GovernorDecision;

use super::{AgentRuntime, StepOutcome};

impl AgentRuntime {
    #[allow(clippy::too_many_arguments)] // TODO(Faz 2): parametre struct'ı
    pub(super) async fn execute_guarded_step(
        &mut self,
        context: &AgentContext,
        step: &AgentPlanStep,
        steps_taken: usize,
        tokens_used: usize,
        objective: &str,
        remaining_steps: &[AgentPlanStep],
        // Otonom döngüde: o ana kadarki geçmiş (onay kaydına yazılır, B6).
        history: Option<&[StepRecord]>,
    ) -> Result<StepOutcome, RuntimeError> {
        debug!(
            step_id = %step.id,
            step_name = %step.name,
            "Executing agent step"
        );

        // ── Reasoning trace ──────────────────────────────
        // NOT: Adım bazlı "bu adımda ne yapmalıyım?" sorgusu artık
        // otonom modda plan_next() üzerinden gerçekten yapılıyor
        // (bkz. run_autonomous_steps). Bu trace, ReasoningTrace tipini
        // besleyen ayrı, hafif bir kayıt — audit log'un yerini tutmaz.
        let _trace = ReasoningTrace {
            agent_id: context.agent_id.to_string(),
            decision: format!("Executing step: {}", step.name),
            timestamp: chrono::Utc::now(),
        };

        // ── Onay ön-kontrolü + Governor kararının denetime yazılması
        // (V10 Sprint 5 + Sprint 6) ─────────────────────────
        let tool_name_for_check: &str = step
            .tool_call
            .as_ref()
            .map(|tc| tc.tool_name.as_str())
            .unwrap_or(step.name.as_str());

        let tool_exists = self.tools.iter().any(|t| t.name() == tool_name_for_check);

        // Denetim için: bu adımda tool'a GERÇEKTEN verilecek argümanlar
        // (invoke_best_tool yolunda argüman adım adıdır). Audit'e
        // yazılırken audit::sanitize_arguments ile maskelenir/kırpılır.
        let call_args: Vec<String> = match &step.tool_call {
            Some(tc) => tc.arguments.clone(),
            None => vec![step.name.clone()],
        };

        if let Some(tool) = self.tools.iter().find(|t| t.name() == tool_name_for_check) {
            let decision = self
                .governor
                .evaluate_call(context.agent_id, tool, &call_args);

            let (decision_label, decision_reason): (&str, Option<String>) = match &decision {
                GovernorDecision::Allow => ("allow", None),
                GovernorDecision::Deny { reason } => ("deny", Some(reason.clone())),
                GovernorDecision::RequiresApproval { reason } => {
                    ("requires_approval", Some(reason.clone()))
                }
            };

            self.audit(
                context.agent_id,
                context.execution_id,
                AuditEventKind::GovernorDecision {
                    tool_name: tool_name_for_check.to_string(),
                    decision: decision_label.to_string(),
                    reason: decision_reason,
                    arguments: call_args.clone(),
                },
            );

            if let GovernorDecision::RequiresApproval { reason } = decision {
                let tool_call = step.tool_call.clone().unwrap_or_else(|| ToolCall {
                    tool_name: tool_name_for_check.to_string(),
                    arguments: vec![],
                });

                let paused_arguments = tool_call.arguments.clone();

                let pending = PendingApproval {
                    id: Uuid::new_v4(),
                    context: context.clone(),
                    objective: objective.to_string(),
                    tool_call,
                    reason: reason.clone(),
                    remaining_steps: remaining_steps.to_vec(),
                    budget: AgentExecutionBudget {
                        max_tokens: self.budget.max_tokens.saturating_sub(tokens_used),
                        max_steps: self.budget.max_steps.saturating_sub(steps_taken),
                        max_runtime_seconds: self.budget.max_runtime_seconds,
                    },
                    created_at: chrono::Utc::now(),
                    autonomous: history.is_some(),
                    history: history
                        .map(|h| h.iter().map(StepSnapshot::from).collect())
                        .unwrap_or_default(),
                };
                let approval_id = pending.id;

                if let Some(store) = &self.approval_store {
                    store.add(pending);
                } else {
                    warn!(
                        agent_id = %context.agent_id,
                        "ApprovalStore bağlanmamış — onay kaydı hiçbir yerde saklanmıyor, kullanıcı bunu asla onaylayamayacak"
                    );
                }

                self.audit(
                    context.agent_id,
                    context.execution_id,
                    AuditEventKind::ExecutionPaused {
                        approval_id,
                        tool_name: tool_name_for_check.to_string(),
                        reason,
                        arguments: paused_arguments,
                    },
                );

                info!(
                    agent_id = %context.agent_id,
                    execution_id = %context.execution_id,
                    approval_id = %approval_id,
                    tool = tool_name_for_check,
                    "Agent execution paused — awaiting approval"
                );

                return Ok(StepOutcome::Paused { approval_id });
            }
        }

        // ── Tool invocation ───────────────────────────────
        let result = if let Some(tool_call) = &step.tool_call {
            self.invoke_tool_call(context.agent_id, tool_call).await
        } else {
            self.invoke_best_tool(context.agent_id, &step.name).await
        };

        match result {
            Ok(output) => {
                if tool_exists {
                    self.audit(
                        context.agent_id,
                        context.execution_id,
                        AuditEventKind::ToolInvoked {
                            tool_name: tool_name_for_check.to_string(),
                            success: true,
                            error: None,
                            arguments: call_args.clone(),
                            output: Some(summarize_output(&output)),
                        },
                    );
                }

                self.memory.store(
                    step.id.to_string(),
                    serde_json::Value::String(output.clone()),
                );

                debug!(
                    step_name = %step.name,
                    output_len = output.len(),
                    "Step completed"
                );

                Ok(StepOutcome::Ran {
                    output,
                    success: true,
                })
            }

            Err(e) => {
                if tool_exists {
                    self.audit(
                        context.agent_id,
                        context.execution_id,
                        AuditEventKind::ToolInvoked {
                            tool_name: tool_name_for_check.to_string(),
                            success: false,
                            error: Some(e.clone()),
                            arguments: call_args.clone(),
                            output: None,
                        },
                    );
                }

                if step.retryable {
                    warn!(
                        step_name = %step.name,
                        error = %e,
                        "Step failed, marked retryable"
                    );
                    Ok(StepOutcome::Ran {
                        output: e,
                        success: false,
                    })
                } else {
                    let message = format!("Agent step '{}' failed: {}", step.name, e);
                    self.audit(
                        context.agent_id,
                        context.execution_id,
                        AuditEventKind::ExecutionFailed {
                            error: message.clone(),
                        },
                    );
                    Err(RuntimeError::TaskExecutionFailed { message })
                }
            }
        }
    }

    /// Capability kontrolü + Risk değerlendirmesi + tool.invoke() —
    /// exact-match ve yapısal ToolCall yolunun paylaştığı tek karar
    /// noktası.
    /// Capability + Risk kararlarını tek bir yerde (SecurityGovernor)
    /// birleştirip tool.invoke()'u çağırır. Karar mantığının tamamı
    /// artık security::governor::SecurityGovernor'da — bkz. o dosya.
    async fn check_and_invoke(
        &self,
        agent_id: Uuid,
        tool: &Arc<dyn AgentTool>,
        arguments: Vec<String>,
    ) -> Result<String, String> {
        match self.governor.evaluate_call(agent_id, tool, &arguments) {
            GovernorDecision::Allow => tool.invoke(arguments).await,
            GovernorDecision::Deny { reason } => Err(format!(
                "tool call denied for '{}': {}",
                tool.name(),
                reason
            )),
            // V10 Sprint 5: normal akışta buraya hiç ulaşılmaz çünkü
            // run_steps() tool'u invoke etmeden ÖNCE bunu yakalayıp
            // duraklatıyor. Bu dal, invoke_tool_call/invoke_best_tool'u
            // run_steps DIŞINDA doğrudan çağıran yerler için (örn.
            // testler) savunma amaçlı — onay mekanizması olmadan
            // "belki" bir eylemi çalıştırmak güvenli değil.
            GovernorDecision::RequiresApproval { reason } => Err(format!(
                "tool call requires approval for '{}' (bu çağrı yolunda onay akışı yok, reddedildi): {}",
                tool.name(),
                reason
            )),
        }
    }

    /// V10 Sprint 2 (Action/Tool Protocol): planner'ın ürettiği yapısal
    /// `ToolCall`'ı çalıştırır. `call.tool_name`, AgentPlanner::ai_plan
    /// tarafından zaten agent'ın gerçek tool listesine karşı doğrulanmış
    /// olsa da, burada tekrar exact-match aranır (savunma amaçlı —
    /// planner ile runtime arasında tool seti değişmiş olabilir).
    /// Eşleşme yoksa (artık bir hallucination değil, gerçek bir
    /// tutarsızlık) sessizce geçmek yerine hata döner.
    pub(crate) async fn invoke_tool_call(
        &self,
        agent_id: Uuid,
        call: &ToolCall,
    ) -> Result<String, String> {
        for tool in &self.tools {
            if tool.name() == call.tool_name {
                return self
                    .check_and_invoke(agent_id, tool, call.arguments.clone())
                    .await;
            }
        }

        Err(format!(
            "ToolCall bilinmeyen bir tool'a işaret ediyor: '{}'",
            call.tool_name
        ))
    }

    /// Adım ismiyle eşleşen tool'u bul ve çağır.
    /// Eşleşme yoksa varsayılan "noop" sonucu döner.
    ///
    /// `pub(crate)`: CapabilityEngine gating'ini planner'ın ürettiği
    /// (ve tool ismiyle asla tam eşleşmesi garanti olmayan) adımlara
    /// bağımlı kalmadan doğrudan ve deterministik test edebilmek için
    /// crate-içi görünür bırakıldı — bkz. src/tests/capability_engine_tests.rs.
    ///
    /// V10 Sprint 2 sonrası bu yol yalnızca tool_call taşımayan adımlar
    /// (fallback_plan'ın "salt muhasebe" adımları) için kullanılır —
    /// asıl tool seçimi artık invoke_tool_call üzerinden, yapısal olarak
    /// yapılıyor.
    pub(crate) async fn invoke_best_tool(
        &self,
        agent_id: Uuid,
        step_name: &str,
    ) -> Result<String, String> {
        // Tool ismi ile step adını eşleştir (fuzzy değil, exact)
        for tool in &self.tools {
            if tool.name() == step_name {
                return self
                    .check_and_invoke(agent_id, tool, vec![step_name.to_string()])
                    .await;
            }
        }

        // Eşleşen tool yok — adımı geç, boş sonuç
        debug!(
            step_name = %step_name,
            "No tool matched for step, skipping"
        );
        Ok(String::new())
    }
}
