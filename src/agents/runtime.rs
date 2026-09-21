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

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::memory::AgentMemory;
use crate::agents::planner::AgentPlanner;
use crate::agents::plans::ToolCall;
use crate::agents::reasoning::ReasoningTrace;
use crate::agents::tools::AgentTool;
use crate::ai::routing::router::ProviderRouter;
use crate::errors::runtime::RuntimeError;
use crate::security::capability_engine::{CapabilityDecision, CapabilityEngine};

pub struct AgentRuntime {
    memory: AgentMemory,
    tools: Vec<Arc<dyn AgentTool>>,
    budget: AgentExecutionBudget,
    ai_router: Option<Arc<ProviderRouter>>,
    capability_engine: Option<Arc<CapabilityEngine>>,
}

impl AgentRuntime {
    pub fn new(
        budget: AgentExecutionBudget,
        tools: Vec<Arc<dyn AgentTool>>,
        ai_router: Option<Arc<ProviderRouter>>,
        capability_engine: Option<Arc<CapabilityEngine>>,
    ) -> Self {
        Self {
            memory: AgentMemory::new(),
            tools,
            budget,
            ai_router,
            capability_engine,
        }
    }

    /// Agent execution döngüsü.
    ///
    /// objective → plan → adım adım çalıştır → execution_id
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
    ) -> Result<Uuid, RuntimeError> {
        info!(
            agent_id = %context.agent_id,
            execution_id = %context.execution_id,
            objective = %objective,
            "Agent execution started"
        );

        let mut runtime = Self::new(budget, tools, ai_router, capability_engine);
        runtime.run(context, objective).await
    }

    async fn run(
        &mut self,
        context: AgentContext,
        objective: String,
    ) -> Result<Uuid, RuntimeError> {
        // ── 1. Plan ───────────────────────────────────────────
        // AI destekli plan; aktif provider yoksa otomatik fallback.
        // V10 Sprint 2: planner artık gerçek tool listesini görüyor —
        // adım isimleriyle tesadüfi eşleşmeye değil, AI'nin BİLEREK
        // seçtiği yapısal bir ToolCall'a dayanıyoruz (bkz. invoke_tool_call).
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
            // NOT: Adım bazlı "bu adımda ne yapmalıyım?" sorgusu
            // henüz yok — şu an plan tek seferde AgentPlanner.plan()
            // ile (aktif ProviderRouter üzerinden) üretiliyor.
            // Adım-bazlı dinamik reasoning gelecek bir fazda eklenebilir.
            let _trace = ReasoningTrace {
                agent_id: context.agent_id.to_string(),
                decision: format!(
                    "Executing step: {}",
                    step.name
                ),
                timestamp: chrono::Utc::now(),
            };

            // ── Tool invocation ───────────────────────────────
            // V10 Sprint 2: adımın yapısal bir ToolCall'ı varsa (AI
            // gerçek tool listesinden bilerek seçti) onu çalıştır.
            // Yoksa eski davranış: adım ismiyle exact-match dene
            // (fallback_plan'ın "salt muhasebe" adımları için).
            let result = if let Some(tool_call) = &step.tool_call {
                self.invoke_tool_call(context.agent_id, tool_call).await
            } else {
                self.invoke_best_tool(context.agent_id, &step.name).await
            };

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

        Ok(context.execution_id)
    }

    /// Capability kontrolü + tool.invoke() — hem exact-match hem de
    /// yapısal ToolCall yolunun paylaştığı tek karar noktası.
    async fn check_and_invoke(
        &self,
        agent_id: Uuid,
        tool: &Arc<dyn AgentTool>,
        arguments: Vec<String>,
    ) -> Result<String, String> {
        if let Some(engine) = &self.capability_engine {
            let decision = engine.check(&agent_id, tool.required_capability());
            if let CapabilityDecision::Denied { reason } = decision {
                warn!(
                    agent_id = %agent_id,
                    tool = tool.name(),
                    reason = %reason,
                    "Tool invocation denied by CapabilityEngine"
                );
                return Err(format!(
                    "capability denied for tool '{}': {}",
                    tool.name(),
                    reason
                ));
            }
        }

        tool.invoke(arguments).await
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
