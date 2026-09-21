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

use crate::agents::plans::{AgentPlan, AgentPlanStep, ToolCall};
use crate::agents::tools::AgentTool;
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
    /// V10 Sprint 2: AI, bu adımda bir tool çağırmak istiyorsa
    /// system prompt'ta verilen isimlerden BİRİNİ buraya yazar.
    /// Yoksa alanı hiç koymaz (serde `default` → None).
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    arguments: Vec<String>,
}

pub struct AgentPlanner;

impl AgentPlanner {
    /// Objective'den plan üret.
    ///
    /// Aktif bir AI provider varsa AI destekli plan (ai_plan).
    /// Yoksa (router yok, hiç provider aktif değil, rate-limit,
    /// parse hatası) fallback_plan.
    ///
    /// `tools`: agent'ın gerçekten erişebileceği tool listesi — AI'ye
    /// SADECE bu isimler sunulur (bkz. ai_plan'ın system prompt'u).
    /// Bu olmadan AI, var olmayan ya da erişilemeyen tool isimleri
    /// üretebilir; CapabilityEngine bunu zaten reddeder ama daha
    /// isabetli olan, AI'nin baştan gerçek seçenekleri bilmesidir.
    pub async fn plan(
        objective: String,
        router: Option<Arc<ProviderRouter>>,
        tools: &[Arc<dyn AgentTool>],
    ) -> AgentPlan {
        if let Some(router) = router {
            if let Some(plan) = Self::ai_plan(objective.clone(), &router, tools).await {
                return plan;
            }
        }
        Self::fallback_plan(objective)
    }

    /// Aktif provider ile objective → structured plan.
    async fn ai_plan(
        objective: String,
        router: &Arc<ProviderRouter>,
        tools: &[Arc<dyn AgentTool>],
    ) -> Option<AgentPlan> {
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name()).collect();

        let system = if tool_names.is_empty() {
            r#"You are an AetherOS agent planner.
Given an objective, output a JSON plan with this exact structure:
{"steps": [{"name": "step_name", "retryable": true}]}
No tools are available right now — never set "tool_name".
Output ONLY valid JSON, no explanation."#
                .to_string()
        } else {
            format!(
                r#"You are an AetherOS agent planner.
Given an objective, output a JSON plan with this exact structure:
{{"steps": [{{"name": "step_name", "retryable": true, "tool_name": "...", "arguments": ["..."]}}]}}

Available tools (use ONLY these exact names in "tool_name", nothing else):
{}

Only include "tool_name" on a step if it should actually invoke that tool.
Steps that are pure reasoning/bookkeeping should omit "tool_name" entirely.
Output ONLY valid JSON, no explanation."#,
                tool_names
                    .iter()
                    .map(|n| format!("- {n}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };

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
            .map(|s| {
                // GÜVENLİK: AI'nin ürettiği tool_name'e körü körüne
                // güvenmiyoruz — sadece gerçekten agent'a verilmiş
                // tool listesindeyse bir ToolCall'a çevriliyor.
                // Uydurma/yanlış isim → sessizce None (adım tool'suz
                // bir adım olarak devam eder, execution durmaz).
                let tool_call = s.tool_name.and_then(|name| {
                    if tool_names.contains(&name.as_str()) {
                        Some(ToolCall {
                            tool_name: name,
                            arguments: s.arguments,
                        })
                    } else {
                        tracing::warn!(
                            hallucinated_tool = %name,
                            "AI, tool listesinde olmayan bir tool ismi üretti — göz ardı edildi"
                        );
                        None
                    }
                });

                AgentPlanStep {
                    id: Uuid::new_v4(),
                    name: s.name,
                    retryable: s.retryable,
                    tool_call,
                }
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
    /// Tool listesinden habersizdir — hiçbir adım tool_call taşımaz.
    fn fallback_plan(objective: String) -> AgentPlan {
        AgentPlan {
            id: Uuid::new_v4(),
            objective: objective.clone(),
            planned_steps: vec![
                AgentPlanStep {
                    id: Uuid::new_v4(),
                    name: "initialize".to_string(),
                    retryable: true,
                    tool_call: None,
                },
                AgentPlanStep {
                    id: Uuid::new_v4(),
                    name: format!(
                        "execute: {}",
                        objective.chars().take(40).collect::<String>()
                    ),
                    retryable: true,
                    tool_call: None,
                },
                AgentPlanStep {
                    id: Uuid::new_v4(),
                    name: "finalize".to_string(),
                    retryable: false,
                    tool_call: None,
                },
            ],
        }
    }
}
