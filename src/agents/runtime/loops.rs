use std::sync::Arc;

use tracing::{info, warn};

use crate::agents::context::AgentContext;
use crate::agents::planner::{is_repeat_of_last, AgentPlanner, NextStepDecision, StepRecord};
use crate::agents::plans::AgentPlanStep;
use crate::ai::routing::router::ProviderRouter;
use crate::errors::runtime::RuntimeError;
use crate::logging::audit::AuditEventKind;

use crate::agents::observability::AgentEventKind;
use crate::agents::state::AgentState;

use super::{AgentOutcome, AgentRuntime, StepOutcome};

const PLAN_RETRIES: usize = 2;

impl AgentRuntime {
    pub(super) async fn run(
        &mut self,
        context: AgentContext,
        objective: String,
    ) -> Result<AgentOutcome, RuntimeError> {
        self.check_cancelled(&context)?;

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
            self.transition(&context, AgentState::Executing, Some("autonomous".into()));
            return self
                .run_autonomous_steps(&context, &objective, router, Vec::new(), 0, 0)
                .await;
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

        self.transition(&context, AgentState::Executing, Some("fixed plan".into()));
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
            self.check_cancelled(context)?;
            self.accounting.steps_used = steps_taken;
            self.accounting.tokens_used = tokens_used;
            self.check_budget_accounting(context)?;

            if steps_taken >= self.budget.max_steps {
                warn!(
                    agent_id = %context.agent_id,
                    max_steps = self.budget.max_steps,
                    "Agent step budget exceeded"
                );
                self.emit(context, AgentEventKind::BudgetExceeded);
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
                    None,
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
    pub(super) async fn run_autonomous_steps(
        &mut self,
        context: &AgentContext,
        objective: &str,
        router: Arc<ProviderRouter>,
        // B6: onaydan sonra devam ederken önceki geçmiş ve harcama taşınır.
        mut history: Vec<StepRecord>,
        mut steps_taken: usize,
        mut tokens_used: usize,
    ) -> Result<AgentOutcome, RuntimeError> {
        loop {
            self.check_cancelled(context)?;
            self.accounting.steps_used = steps_taken;
            self.accounting.tokens_used = tokens_used;
            self.check_budget_accounting(context)?;

            if steps_taken >= self.budget.max_steps {
                warn!(
                    agent_id = %context.agent_id,
                    max_steps = self.budget.max_steps,
                    "Agent step budget exceeded (autonomous loop)"
                );
                self.emit(context, AgentEventKind::BudgetExceeded);
                break;
            }

            if tokens_used >= self.budget.max_tokens {
                warn!(
                    agent_id = %context.agent_id,
                    max_tokens = self.budget.max_tokens,
                    "Agent token budget exceeded (autonomous loop)"
                );
                self.emit(context, AgentEventKind::BudgetExceeded);
                break;
            }

            // Yalnız yetkisi olan araçlar planlayıcıya gösterilir.
            let usable = self.usable_tools(context.agent_id);
            if history.is_empty()
                && !self.tools.is_empty()
                && usable.is_empty()
                && router.active_id().is_some()
            {
                let message = "Bu agent'a hiçbir araç yetkisi verilmemiş, bir şey yapamaz. \
                               Agent'ı başlatırken Yetkiler bölümünden gerekli çipleri seç \
                               (ör. Terminal, Dosya okuma)."
                    .to_string();
                return Err(self.fail_execution(context, message));
            }

            // Geçici hatalara (limit, bağlantı, bozuk yanıt) karşı kısa yeniden deneme.
            let mut decision: Option<NextStepDecision> = None;
            let mut plan_error = String::new();
            for attempt in 0..=PLAN_RETRIES {
                match AgentPlanner::plan_next_detailed(objective, &history, &router, &usable).await {
                    Ok(d) => {
                        decision = Some(d);
                        break;
                    }
                    Err(reason) => {
                        plan_error = reason;
                        if router.active_id().is_none() {
                            break; // sağlayıcı yok: tekrar denemenin anlamı yok
                        }
                        if attempt < PLAN_RETRIES {
                            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                        }
                    }
                }
            }

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
                None => {
                    // Planlayıcıdan karar ALINAMADI. Eskiden burada sabit,
                    // boş bir plana düşülüp "completed" deniyordu — hiçbir
                    // şey yapılmadığı hâlde başarı görünüyordu. Artık
                    // dürüstçe başarısız sayılır ve sebep kullanıcıya gider.
                    let message = if history.is_empty() {
                        if router.active_id().is_none() {
                            "AI sağlayıcı aktif değil: agent hedefi planlayamaz. \
                             Ayarlar → AI'dan bir sağlayıcı ekleyip aktif et."
                                .to_string()
                        } else {
                            format!("AI sağlayıcıdan geçerli bir plan alınamadı. {plan_error}")
                        }
                    } else {
                        format!(
                            "{} adım başarıyla çalıştı ama AI sonraki adımı belirleyemedi \
                             (hedefin bittiği doğrulanamadı). {plan_error}",
                            history.len()
                        )
                    };
                    warn!(
                        agent_id = %context.agent_id,
                        reason = %message,
                        "Autonomous loop: plan alınamadı, execution başarısız sayılıyor"
                    );
                    return Err(self.fail_execution(context, message));
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
                .execute_guarded_step(
                    context,
                    &step,
                    steps_taken,
                    tokens_used,
                    objective,
                    &[],
                    Some(history.as_slice()),
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
                    history.push(StepRecord {
                        step_name: step.name.clone(),
                        success,
                        output,
                        tool_call: step.tool_call.clone(),
                    });
                    steps_taken += 1;

                    // Aynı çağrı arka arkaya iki kez BAŞARISIZ olduysa üçüncü kez denemek
                    // boşa token harcar; dürüstçe durdur.
                    if let [.., prev, last] = history.as_slice() {
                        if !prev.success
                            && !last.success
                            && last.tool_call.is_some()
                            && prev.tool_call == last.tool_call
                        {
                            let err: String = last.output.chars().take(300).collect();
                            return Err(self.fail_execution(
                                context,
                                format!("Aynı çağrı iki kez başarısız oldu, durduruldu. Son hata: {err}"),
                            ));
                        }
                    }
                }
            }
        }

        // Adım çalıştırıldı ama HİÇBİRİ başarılı olmadıysa "tamamlandı"
        // demek yanıltıcıdır.
        if !history.is_empty() && !history.iter().any(|r| r.success) {
            let last: String = history
                .last()
                .map(|r| r.output.chars().take(300).collect())
                .unwrap_or_default();
            return Err(self.fail_execution(
                context,
                format!("Hiçbir adım başarılı olmadı. Son hata: {last}"),
            ));
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
}
