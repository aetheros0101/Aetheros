// ============================================================
// src/agents/runtime.rs
//
// Faz 5 Düzeltmesi: Gerçek Agent Execution Döngüsü
//
// ÖNCE: execute() → context.execution_id döndür (stub)
//
// SONRA: Plan → Step döngüsü → Tool invocation
//
//   1. AgentPlanner'dan plan al (objective → adımlar)
//   2. Her adım için:
//      a. Budget kontrolü (token + step limiti)
//      b. Reasoning trace yaz
//      c. Tool invocation (araç çağrısı)
//      d. Sonucu memory'e kaydet
//   3. Tüm adımlar bitti → execution_id döndür
//
// AI entegrasyonu (Faz 6'da tamamlanacak):
//   Şu an: planner sabit plan üretiyor
//   Faz 6: AnthropicProvider.infer() ile dynamic planning
// ============================================================

use std::sync::Arc;

use tracing::{
    debug,
    info,
    warn,
};
use uuid::Uuid;

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::memory::AgentMemory;
use crate::agents::planner::AgentPlanner;
use crate::agents::reasoning::ReasoningTrace;
use crate::agents::tools::AgentTool;
use crate::ai::providers::provider::ModelProvider;
use crate::errors::runtime::RuntimeError;

pub struct AgentRuntime {
    memory: AgentMemory,
    tools: Vec<Arc<dyn AgentTool>>,
    budget: AgentExecutionBudget,
    /// Faz 6: configure_ai_provider ile ayarlanmış sağlayıcı.
    /// None ise AgentPlanner eski env-var davranışına düşer.
    ai_provider: Option<Arc<dyn ModelProvider>>,
}

impl AgentRuntime {
    pub fn new(
        budget: AgentExecutionBudget,
        tools: Vec<Arc<dyn AgentTool>>,
        ai_provider: Option<Arc<dyn ModelProvider>>,
    ) -> Self {
        Self {
            memory: AgentMemory::new(),
            tools,
            budget,
            ai_provider,
        }
    }

    /// Agent execution döngüsü.
    ///
    /// objective → plan → adım adım çalıştır → (execution_id, plan adım isimleri)
    pub async fn execute(
        context: AgentContext,
        objective: String,
        budget: AgentExecutionBudget,
        tools: Vec<Arc<dyn AgentTool>>,
        ai_provider: Option<Arc<dyn ModelProvider>>,
    ) -> Result<(Uuid, Vec<String>), RuntimeError> {
        info!(
            agent_id = %context.agent_id,
            execution_id = %context.execution_id,
            objective = %objective,
            "Agent execution started"
        );

        let mut runtime = Self::new(budget, tools, ai_provider);
        runtime.run(context, objective).await
    }

    async fn run(
        &mut self,
        context: AgentContext,
        objective: String,
    ) -> Result<(Uuid, Vec<String>), RuntimeError> {
        // ── 1. Plan ───────────────────────────────────────────
        // AI destekli plan; API yoksa otomatik fallback.
        let plan =
            AgentPlanner::plan(objective.clone(), self.ai_provider.clone()).await;

        let step_names: Vec<String> =
            plan.planned_steps.iter().map(|s| s.name.clone()).collect();

        info!(
            plan_id = %plan.id,
            steps = plan.planned_steps.len(),
            "Agent plan created"
        );

        let mut steps_taken = 0;
        let mut tokens_used = 0;

        // ── 2. Adım döngüsü ──────────────────────────────────
        for step in &plan.planned_steps {
            // Budget: adım limiti kontrolü
            if steps_taken >= self.budget.max_steps {
                warn!(
                    agent_id = %context.agent_id,
                    max_steps = self.budget.max_steps,
                    "Agent step budget exceeded"
                );
                break;
            }

            // Budget: token limiti kontrolü
            if tokens_used >= self.budget.max_tokens {
                warn!(
                    agent_id = %context.agent_id,
                    max_tokens = self.budget.max_tokens,
                    "Agent token budget exceeded"
                );
                break;
            }

            debug!(
                step_id = %step.id,
                step_name = %step.name,
                "Executing agent step"
            );

            // ── Reasoning trace ──────────────────────────────
            // Faz 6: Bu noktada AnthropicProvider'dan
            // "bu adımda ne yapmalıyım?" sorusu sorulacak.
            let _trace = ReasoningTrace {
                agent_id: context.agent_id.to_string(),
                decision: format!(
                    "Executing step: {}",
                    step.name
                ),
                timestamp: chrono::Utc::now(),
            };

            // ── Tool invocation ───────────────────────────────
            let result = self
                .invoke_best_tool(&step.name)
                .await;

            match result {
                Ok(output) => {
                    // Sonucu memory'e kaydet
                    self.memory.store(
                        step.id.to_string(),
                        serde_json::Value::String(
                            output.clone(),
                        ),
                    );

                    // Yaklaşık token sayımı (Faz 6'da API'den gelecek)
                    tokens_used +=
                        output.split_whitespace().count();

                    debug!(
                        step_name = %step.name,
                        output_len = output.len(),
                        "Step completed"
                    );
                }

                Err(e) => {
                    if step.retryable {
                        warn!(
                            step_name = %step.name,
                            error = %e,
                            "Step failed, marked retryable"
                        );
                        // Faz 6: retry mantığı buraya
                    } else {
                        return Err(
                            RuntimeError::TaskExecutionFailed {
                                message: format!(
                                    "Agent step '{}' failed: {}",
                                    step.name, e
                                ),
                            },
                        );
                    }
                }
            }

            steps_taken += 1;
        }

        info!(
            agent_id = %context.agent_id,
            execution_id = %context.execution_id,
            steps_taken,
            tokens_used,
            "Agent execution complete"
        );

        Ok((context.execution_id, step_names))
    }

    /// Adım ismiyle eşleşen tool'u bul ve çağır.
    /// Eşleşme yoksa varsayılan "noop" sonucu döner.
    async fn invoke_best_tool(
        &self,
        step_name: &str,
    ) -> Result<String, String> {
        // Tool ismi ile step adını eşleştir (fuzzy değil, exact)
        for tool in &self.tools {
            if tool.name() == step_name {
                return tool
                    .invoke(vec![step_name.to_string()])
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
