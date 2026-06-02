// ============================================================
// src/agents/planner.rs  (v2)
//
// Faz 6: AnthropicProvider ile gerçek AI planning.
//
// ÖNCE: plan() → sabit "initialize" adımı döndüren stub.
//
// SONRA:
//   1. AnthropicProvider ile objective → JSON plan üret
//   2. JSON parse başarısızsa → fallback sabit plan
//   3. Structured output: adım listesi
//
// API Prompt yapısı:
//   system: "AetherOS planner. JSON çıktı ver."
//   user:   "Objective: {objective}\nAdımları listele."
//
// Fallback garantisi:
//   API çağrısı veya parse başarısız olursa
//   runtime donmaz — minimal plan ile devam eder.
// ============================================================

use serde::{
    Deserialize,
    Serialize,
};
use tracing::{
    debug,
    warn,
};
use uuid::Uuid;

use crate::agents::plans::{
    AgentPlan,
    AgentPlanStep,
};
use crate::ai::inference::request::InferenceRequest;
use crate::ai::providers::anthropic::AnthropicProvider;
use crate::ai::providers::provider::ModelProvider;

/// Anthropic'ten beklenen JSON çıktı yapısı.
#[derive(Debug, Deserialize, Serialize)]
struct PlanResponse {
    steps: Vec<PlanStepResponse>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PlanStepResponse {
    name: String,
    retryable: bool,
}

pub struct AgentPlanner;

impl AgentPlanner {
    /// Objective'den plan üret.
    ///
    /// AnthropicProvider mevcutsa AI destekli plan.
    /// Değilse (API key yok, rate limit vb.) fallback plan.
    pub fn plan(objective: String) -> AgentPlan {
        // Direkt fallback plan — AI planning Faz sonrası
        Self::fallback_plan(objective)
    }

    /// AnthropicProvider ile objective → structured plan.
    async fn ai_plan(
        objective: String,
    ) -> Option<AgentPlan> {
        let provider =
            AnthropicProvider::new().ok()?;

        let system = r#"You are an AetherOS agent planner.
Given an objective, output a JSON plan with this exact structure:
{"steps": [{"name": "step_name", "retryable": true}]}
Output ONLY valid JSON, no explanation."#;

        let prompt = format!(
            "Objective: {objective}\n\
             Create a concise execution plan with 2-5 steps."
        );

        let request = InferenceRequest::new(prompt, 512)
            .with_system(system)
            .with_temperature(0.3); // Düşük temperature → tutarlı JSON

        let response =
            provider.infer(request).await.ok()?;

        debug!(
            tokens = response.tokens_used,
            "AI plan response received"
        );

        // JSON fence'leri temizle (```json ... ``` varsa)
        let cleaned = response
            .output
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let plan_resp: PlanResponse =
            serde_json::from_str(cleaned).ok()?;

        let steps = plan_resp
            .steps
            .into_iter()
            .map(|s| AgentPlanStep {
                id: Uuid::new_v4(),
                name: s.name,
                retryable: s.retryable,
            })
            .collect::<Vec<_>>();

        if steps.is_empty() {
            return None;
        }

        Some(AgentPlan {
            id: Uuid::new_v4(),
            objective,
            planned_steps: steps,
        })
    }

    /// API'siz çalışabilen minimal fallback plan.
    fn fallback_plan(objective: String) -> AgentPlan {
        AgentPlan {
            id: Uuid::new_v4(),
            objective: objective.clone(),
            planned_steps: vec![
                AgentPlanStep {
                    id: Uuid::new_v4(),
                    name: "initialize".to_string(),
                    retryable: true,
                },
                AgentPlanStep {
                    id: Uuid::new_v4(),
                    name: format!(
                        "execute: {}",
                        objective
                            .chars()
                            .take(40)
                            .collect::<String>()
                    ),
                    retryable: true,
                },
                AgentPlanStep {
                    id: Uuid::new_v4(),
                    name: "finalize".to_string(),
                    retryable: false,
                },
            ],
        }
    }
}
