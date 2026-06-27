// ============================================================
// src/agents/planner.rs  (v3)
//
// plan() artık async. ai_plan() önce denenir;
// API key yoksa, rate-limit varsa veya parse başarısızsa
// fallback_plan() devreye girer → runtime hiç donmaz.
// ============================================================

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use crate::agents::plans::{AgentPlan, AgentPlanStep};
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
    /// `provider` verilmişse (mobil: configure_ai_provider ile
    /// register edilmiş Gemini/Anthropic) o kullanılır.
    /// `None` ise eski davranış: ANTHROPIC_API_KEY env var'ı dener.
    /// İkisi de başarısız olursa (API key yok, rate-limit, parse
    /// hatası) fallback_plan devreye girer.
    pub async fn plan(
        objective: String,
        provider: Option<Arc<dyn ModelProvider>>,
    ) -> AgentPlan {
        match Self::ai_plan(objective.clone(), provider).await {
            Some(plan) => plan,
            None => Self::fallback_plan(objective),
        }
    }

    /// Verilen (veya env var'dan kurulan) provider ile objective → structured plan.
    async fn ai_plan(
        objective: String,
        provider: Option<Arc<dyn ModelProvider>>,
    ) -> Option<AgentPlan> {
        let provider: Arc<dyn ModelProvider> = match provider {
            Some(p) => p,
            None => Arc::new(AnthropicProvider::new().ok()?),
        };

        let system = r#"You are an AetherOS agent planner.
Given an objective, output a JSON plan with this exact structure:
{"steps": [{"name": "step_name", "retryable": true}]}
Output ONLY valid JSON, no explanation."#;

        let prompt = format!(
            "Objective: {objective}\nCreate a concise execution plan with 2-5 steps."
        );

        let request = InferenceRequest::new(prompt, 512)
            .with_system(system)
            .with_temperature(0.3); // Düşük temperature → tutarlı JSON

        let response = provider.infer(request).await.ok()?;

        debug!(tokens = response.tokens_used, "AI plan response received");

        // JSON fence'leri temizle (```json ... ``` varsa)
        let cleaned = response
            .output
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let plan_resp: PlanResponse = serde_json::from_str(cleaned).ok()?;

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
                        objective.chars().take(40).collect::<String>()
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
