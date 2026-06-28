// ============================================================
// src/agents/planner.rs  (v4 — Haziran 2026)
//
// plan() artık async ve ortak ProviderRouter'ı parametre olarak alır.
// ai_plan() önce denenir; router yoksa, aktif provider yoksa,
// rate-limit varsa veya parse başarısızsa fallback_plan() devreye
// girer → runtime hiç donmaz.
//
// ÖNCE (v3): AnthropicProvider::new() doğrudan burada inşa
// ediliyordu (env var'a bağımlıydı, kullanıcının Ayarlar'dan
// seçtiği provider'ı yok sayıyordu).
//
// SONRA: hangi provider'ın aktif olduğuna ProviderRouter karar
// verir — kullanıcı Ayarlar'da hangi modeli aktif ettiyse o kullanılır.
// ============================================================

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use crate::agents::plans::{AgentPlan, AgentPlanStep};
use crate::ai::inference::request::InferenceRequest;
use crate::ai::routing::router::ProviderRouter;

/// Aktif provider'dan beklenen JSON çıktı yapısı.
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
    /// Aktif bir AI provider varsa AI destekli plan (ai_plan).
    /// Yoksa (router yok, hiç provider aktif değil, rate-limit,
    /// parse hatası) fallback_plan.
    pub async fn plan(
        objective: String,
        router: Option<Arc<ProviderRouter>>,
    ) -> AgentPlan {
        if let Some(router) = router {
            if let Some(plan) = Self::ai_plan(objective.clone(), &router).await {
                return plan;
            }
        }
        Self::fallback_plan(objective)
    }

    /// Aktif provider ile objective → structured plan.
    async fn ai_plan(
        objective: String,
        router: &Arc<ProviderRouter>,
    ) -> Option<AgentPlan> {
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

        let response = router.infer_active(request).await.ok()?;

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
