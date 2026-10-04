// ============================================================
// src/agents/runtime.rs
//
// Faz 5 Düzeltmesi: Gerçek Agent Execution Döngüsü
//
// V10 Sprint 5 (Approval Engine): SecurityGovernor bir ToolCall için
// RequiresApproval derse, execution artık hatayla bitmiyor —
// invoke ETMEDEN duraklatılıyor, kalan adımlar + budget bir
// PendingApproval'a yazılıyor (bkz. agents::approval), ve
// AgentOutcome::PendingApproval olarak dönüyor. Kullanıcı onaylarsa
// resume() ile kaldığı yerden devam eder; reddederse hiç çalışmaz.
//
// SONRA: Plan → Step döngüsü → Tool invocation
//
//   1. AgentPlanner'dan plan al (objective → adımlar)
//   2. Her adım için:
//      a. Budget kontrolü (token + step limiti)
//      b. Reasoning trace yaz
//      c. Governor'a danış — RequiresApproval ise DURAKLAT
//      d. Tool invocation (araç çağrısı)
//      e. Sonucu memory'e kaydet
//   3. Tüm adımlar bitti → AgentOutcome::Completed
//
// AI entegrasyonu (Haziran 2026 itibarıyla tamamlandı):
//   AgentPlanner artık ProviderRouter üzerinden kullanıcının
//   Ayarlar'da aktif ettiği provider'ı kullanır (Anthropic/OpenAI/
//   Gemini/Ollama). Router yoksa veya aktif provider yoksa
//   fallback_plan'a düşer.
// ============================================================

use std::sync::Arc;

use tracing::{
    debug,
    info,
    warn,
};
use uuid::Uuid;

use crate::agents::approval::{ApprovalStore, PendingApproval};
use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::memory::AgentMemory;
use crate::agents::planner::{is_repeat_of_last, AgentPlanner, NextStepDecision, StepRecord};
use crate::agents::plans::{AgentPlanStep, ToolCall};
use crate::agents::reasoning::ReasoningTrace;
use crate::agents::tools::AgentTool;
use crate::ai::routing::router::ProviderRouter;
use crate::errors::runtime::RuntimeError;
use crate::logging::audit::{summarize_output, AuditEventKind, AuditLog};
use crate::security::capability_engine::CapabilityEngine;
use crate::security::governor::{GovernorDecision, SecurityGovernor};
use crate::security::risk_engine::RiskEngine;

// V10 Sprint 4: HighRiskPolicy artık security::governor'da yaşıyor
// (SecurityGovernor'ın kendi politikası). Buradaki re-export, mevcut
// `crate::agents::runtime::HighRiskPolicy` importlarının kırılmaması
// için — agents::tools'un types::agent_tool'u re-export ettiği
// desenin aynısı.
pub use crate::security::governor::HighRiskPolicy;

/// Bir agent execution'ının (ya da resume'unun) sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentOutcome {
    /// Plan sonuna kadar (ya da budget bitene kadar) hatasız tamamlandı.
    Completed,
    /// Bir adım SecurityGovernor'dan RequiresApproval aldı — tool hiç
    /// invoke edilmeden execution duraklatıldı. `approval_id`,
    /// ApprovalStore'daki kaydı gösterir (bkz. respond_to_approval).
    PendingApproval { approval_id: Uuid },
}

/// V10 Sprint 7: `execute_guarded_step`'in TEK bir adım için sonucu.
/// `AgentOutcome`'dan farklı — bu, bir adımın sonucu; `AgentOutcome`
/// tüm execution'ın sonucu.
#[derive(Debug)]
enum StepOutcome {
    /// Adım gerçekten çalıştı (başarılı ya da retryable bir hatayla).
    Ran { output: String, success: bool },
    /// Governor RequiresApproval dedi — invoke edilmeden duraklatıldı.
    Paused { approval_id: Uuid },
}

pub struct AgentRuntime {
    memory: AgentMemory,
    tools: Vec<Arc<dyn AgentTool>>,
    budget: AgentExecutionBudget,
    ai_router: Option<Arc<ProviderRouter>>,
    /// V10 Sprint 4: Capability + Risk kararları artık tek bir
    /// SecurityGovernor üzerinden alınıyor — bkz. check_and_invoke.
    governor: SecurityGovernor,
    /// V10 Sprint 5: RequiresApproval durumunda duraklatma kaydının
    /// yazılacağı yer. None ise (eski/basit kullanım) execution yine
    /// duraklatılır ama kayıt hiçbir yerde saklanmaz — kullanıcı asla
    /// onaylayamaz, fiilen kalıcı ret gibi davranır.
    approval_store: Option<Arc<ApprovalStore>>,
    /// V10 Sprint 6: Governor kararlarının, tool çağrılarının ve
    /// pause/resume olaylarının yazıldığı denetim izi. None ise
    /// (eski/basit kullanım) hiçbir şey kaydedilmez — davranış
    /// değişmez, sadece iz kalmaz.
    audit_log: Option<Arc<AuditLog>>,
}

impl AgentRuntime {
    pub fn new(
        budget: AgentExecutionBudget,
        tools: Vec<Arc<dyn AgentTool>>,
        ai_router: Option<Arc<ProviderRouter>>,
        capability_engine: Option<Arc<CapabilityEngine>>,
        risk_engine: Option<Arc<RiskEngine>>,
        approval_store: Option<Arc<ApprovalStore>>,
        audit_log: Option<Arc<AuditLog>>,
    ) -> Self {
        Self {
            memory: AgentMemory::new(),
            tools,
            budget,
            ai_router,
            governor: SecurityGovernor::new(capability_engine, risk_engine),
            approval_store,
            audit_log,
        }
    }

    /// Denetim izine tek bir olay yaz. audit_log bağlanmamışsa (None)
    /// sessizce hiçbir şey yapmaz.
    fn audit(&self, agent_id: Uuid, execution_id: Uuid, kind: AuditEventKind) {
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
    /// runtime hiç donmaz).
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
        runtime.run(context, objective).await
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

        runtime.audit(
            pending.context.agent_id,
            pending.context.execution_id,
            AuditEventKind::ExecutionResumed { approval_id: pending.id },
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
                runtime.memory.store(
                    pending.id.to_string(),
                    serde_json::Value::String(output),
                );
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
                    AuditEventKind::ExecutionFailed { error: message.clone() },
                );
                return Err(RuntimeError::TaskExecutionFailed { message });
            }
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

    async fn run(
        &mut self,
        context: AgentContext,
        objective: String,
    ) -> Result<AgentOutcome, RuntimeError> {
        // V10 Sprint 7 (Autonomous Agent Loop): gerçek bir AI provider
        // varsa OTONOM döngü kullanılır — her adımdan sonra sonuca bakıp
        // yeniden planlanır (bkz. run_autonomous_steps). AI yoksa
        // (fallback_plan zaten sabit ve uyarlanamaz olduğundan) eski
        // sabit plan → run_steps yoluna düşülür; davranış hiç değişmez.
        if let Some(router) = self.ai_router.clone() {
            info!(
                agent_id = %context.agent_id,
                execution_id = %context.execution_id,
                "Starting autonomous agent loop"
            );
            return self.run_autonomous_steps(&context, &objective, router).await;
        }

        let plan = AgentPlanner::plan(
            objective.clone(),
            self.ai_router.clone(),
            &self.tools,
        )
        .await;

        info!(
            plan_id = %plan.id,
            steps = plan.planned_steps.len(),
            "Agent plan created"
        );

        self.run_steps(&context, &plan.planned_steps, 0, 0, &objective)
            .await
    }

    /// V10 Sprint 7: sabit bir planı (fallback_plan ya da AI'siz
    /// yeniden devam eden bir resume) baştan sona yürütür. Her adım
    /// execute_guarded_step üzerinden geçer — guard mantığı
    /// run_autonomous_steps ile paylaşılıyor, iki yerde ayrı ayrı
    /// yazılıp birbirinden sapmasın diye.
    ///
    /// `pub(crate)`: planner'ın (AI'siz ortamda) tool_call üretmeyen
    /// fallback_plan'ına bağımlı kalmadan duraklatma davranışını
    /// doğrudan ve deterministik test edebilmek için — bkz.
    /// src/tests/approval_engine_tests.rs.
    pub(crate) async fn run_steps(
        &mut self,
        context: &AgentContext,
        steps: &[AgentPlanStep],
        mut steps_taken: usize,
        mut tokens_used: usize,
        objective: &str,
    ) -> Result<AgentOutcome, RuntimeError> {
        for (idx, step) in steps.iter().enumerate() {
            if steps_taken >= self.budget.max_steps {
                warn!(
                    agent_id = %context.agent_id,
                    max_steps = self.budget.max_steps,
                    "Agent step budget exceeded"
                );
                break;
            }

            if tokens_used >= self.budget.max_tokens {
                warn!(
                    agent_id = %context.agent_id,
                    max_tokens = self.budget.max_tokens,
                    "Agent token budget exceeded"
                );
                break;
            }

            let outcome = self
                .execute_guarded_step(
                    context,
                    step,
                    steps_taken,
                    tokens_used,
                    objective,
                    &steps[idx + 1..],
                )
                .await?;

            match outcome {
                StepOutcome::Paused { approval_id } => {
                    return Ok(AgentOutcome::PendingApproval { approval_id });
                }
                StepOutcome::Ran { output, success } => {
                    if success {
                        tokens_used += output.split_whitespace().count();
                    }
                    steps_taken += 1;
                }
            }
        }

        self.audit(
            context.agent_id,
            context.execution_id,
            AuditEventKind::ExecutionCompleted,
        );

        info!(
            agent_id = %context.agent_id,
            execution_id = %context.execution_id,
            steps_taken,
            tokens_used,
            "Agent execution complete"
        );

        Ok(AgentOutcome::Completed)
    }

    /// V10 Sprint 7 (Autonomous Agent Loop, guarded): sabit bir planı
    /// körü körüne yürütmek yerine, her adımdan SONRA planner'a "şimdi
    /// ne yapmalıyım?" diye tekrar sorar — sonucu gözlemleyip yeniden
    /// düşünmesi budur. Her adım YİNE execute_guarded_step üzerinden
    /// geçer: Governor/Approval/Audit zinciri run_steps ile BİREBİR
    /// aynı — "otonom" olan sadece planın önceden değil adım adım
    /// kurulması, güvenlik katmanı hiç atlanmıyor.
    ///
    /// BİLİNEN SINIRLAMA: bir adım RequiresApproval alırsa execution
    /// duraklar (tıpkı run_steps gibi). Ama resume() sonrası otonom
    /// döngü DEVAM ETMİYOR — onaylanan adım çalıştırılır ve execution
    /// tamamlanmış sayılır. Otonom modda "duraklat → onayla → yeniden
    /// planlayarak devam et" tam desteği ayrı bir sprint.
    async fn run_autonomous_steps(
        &mut self,
        context: &AgentContext,
        objective: &str,
        router: Arc<ProviderRouter>,
    ) -> Result<AgentOutcome, RuntimeError> {
        let mut steps_taken = 0usize;
        let mut tokens_used = 0usize;
        let mut history: Vec<StepRecord> = Vec::new();

        loop {
            if steps_taken >= self.budget.max_steps {
                warn!(
                    agent_id = %context.agent_id,
                    max_steps = self.budget.max_steps,
                    "Agent step budget exceeded (autonomous loop)"
                );
                break;
            }

            if tokens_used >= self.budget.max_tokens {
                warn!(
                    agent_id = %context.agent_id,
                    max_tokens = self.budget.max_tokens,
                    "Agent token budget exceeded (autonomous loop)"
                );
                break;
            }

            let decision =
                AgentPlanner::plan_next(objective, &history, &router, &self.tools).await;

            let step = match decision {
                Some(NextStepDecision::Done) => {
                    info!(
                        agent_id = %context.agent_id,
                        execution_id = %context.execution_id,
                        "Autonomous loop: objective tamamlandı (AI bildirdi)"
                    );
                    break;
                }
                Some(NextStepDecision::Step(step)) => step,
                None if history.is_empty() => {
                    // AI otonom döngüyü hiç BAŞLATAMADI (muhtemelen aktif
                    // provider yok) — eski güvenlik ağına düş: sabit
                    // fallback_plan. Bu, "ai_router var ama aktif provider
                    // yok" durumunda eskiden olduğu gibi en azından birkaç
                    // no-op adımın çalışmasını korur.
                    warn!(
                        agent_id = %context.agent_id,
                        "Autonomous loop: plan_next ilk denemede başarısız oldu, sabit fallback plana düşülüyor"
                    );
                    let plan = AgentPlanner::plan(
                        objective.to_string(),
                        Some(router.clone()),
                        &self.tools,
                    )
                    .await;
                    return self
                        .run_steps(context, &plan.planned_steps, steps_taken, tokens_used, objective)
                        .await;
                }
                None => {
                    // Otonom döngü BAŞLADI (en az bir adım gerçekten
                    // çalıştı) ama AI şimdi cevap veremiyor (rate limit,
                    // geçici hata vb.) — kısmi ilerlemeyi sabit bir plana
                    // zorlamak yerine burada güvenle bitiriyoruz.
                    warn!(
                        agent_id = %context.agent_id,
                        "Autonomous loop: plan_next başarısız oldu, execution sonlandırılıyor"
                    );
                    break;
                }
            };

            // Planlayıcı, az önce BAŞARIYLA çalışan çağrının aynısını yine
            // istiyorsa (ör. `ls` sonrası tekrar `ls`) hedef zaten
            // karşılanmıştır; tekrar çalıştırmak boşa iş + token.
            if is_repeat_of_last(&history, &step) {
                info!(
                    agent_id = %context.agent_id,
                    execution_id = %context.execution_id,
                    step = %step.name,
                    "Autonomous loop: aynı çağrının ardışık tekrarı engellendi, tamamlandı sayılıyor"
                );
                break;
            }

            // Otonom modda "kalan adımlar" kavramı yok — sonraki adım
            // henüz planlanmadı (bkz. yukarıdaki BİLİNEN SINIRLAMA notu).
            let outcome = self
                .execute_guarded_step(context, &step, steps_taken, tokens_used, objective, &[])
                .await?;

            match outcome {
                StepOutcome::Paused { approval_id } => {
                    return Ok(AgentOutcome::PendingApproval { approval_id });
                }
                StepOutcome::Ran { output, success } => {
                    if success {
                        tokens_used += output.split_whitespace().count();
                    }
                    history.push(StepRecord {
                        step_name: step.name.clone(),
                        success,
                        output,
                        tool_call: step.tool_call.clone(),
                    });
                    steps_taken += 1;
                }
            }
        }

        self.audit(
            context.agent_id,
            context.execution_id,
            AuditEventKind::ExecutionCompleted,
        );

        info!(
            agent_id = %context.agent_id,
            execution_id = %context.execution_id,
            steps_taken,
            tokens_used,
            "Autonomous agent execution complete"
        );

        Ok(AgentOutcome::Completed)
    }

    /// Tek bir adımın TÜM guard mantığı: Governor precheck (gerekirse
    /// duraklatma + PendingApproval + audit), sonra tool invocation +
    /// audit. `run_steps` ve `run_autonomous_steps` bu tek metodu
    /// paylaşır.
    ///
    /// `remaining_steps`: RequiresApproval çıkarsa PendingApproval'a
    /// yazılacak "sonrası" — sabit planda gerçek kalan adımlar, otonom
    /// döngüde her zaman boş dilim (`&[]`).
    async fn execute_guarded_step(
        &mut self,
        context: &AgentContext,
        step: &AgentPlanStep,
        steps_taken: usize,
        tokens_used: usize,
        objective: &str,
        remaining_steps: &[AgentPlanStep],
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
            let decision = self.governor.evaluate_call(context.agent_id, tool, &call_args);

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

                Ok(StepOutcome::Ran { output, success: true })
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
                    Ok(StepOutcome::Ran { output: e, success: false })
                } else {
                    let message = format!("Agent step '{}' failed: {}", step.name, e);
                    self.audit(
                        context.agent_id,
                        context.execution_id,
                        AuditEventKind::ExecutionFailed { error: message.clone() },
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
