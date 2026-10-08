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

use crate::agents::approval::StepSnapshot;
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

/// V10 Sprint 7: `plan_next`'e geçmişte atılmış bir adımın SONUCUNU
/// bildirmek için. `AgentRuntime` her adımdan sonra bunu doldurup
/// history'ye ekler.
#[derive(Debug, Clone)]
pub struct StepRecord {
    pub step_name: String,
    pub success: bool,
    pub output: String,
    /// Adımın çağırdığı tool + argümanlar (varsa). Planlayıcıya GERÇEKTEN
    /// ne çalıştırıldığını gösterir; aynı çağrının tekrarını da buradan
    /// yakalarız (bkz. `is_repeat_of_last`).
    pub tool_call: Option<ToolCall>,
}

impl From<&StepRecord> for StepSnapshot {
    fn from(r: &StepRecord) -> Self {
        StepSnapshot {
            step_name: r.step_name.clone(),
            success: r.success,
            output: r.output.clone(),
            tool_call: r.tool_call.clone(),
        }
    }
}

impl From<&StepSnapshot> for StepRecord {
    fn from(s: &StepSnapshot) -> Self {
        StepRecord {
            step_name: s.step_name.clone(),
            success: s.success,
            output: s.output.clone(),
            tool_call: s.tool_call.clone(),
        }
    }
}

/// Planlayıcının yeni adımı, hemen önceki BAŞARILI adımın birebir aynı
/// tool çağrısı mı? Öyleyse tekrar çalıştırmak yalnızca boşa iş/token
/// harcar (ör. `ls` çıktısını gördükten sonra yine `ls`). Ardışık
/// olmayan tekrarlar (ls → touch → ls) meşrudur ve engellenmez.
pub fn is_repeat_of_last(history: &[StepRecord], next: &AgentPlanStep) -> bool {
    match (history.last(), next.tool_call.as_ref()) {
        (Some(last), Some(call)) => last.success && last.tool_call.as_ref() == Some(call),
        _ => false,
    }
}

/// `plan_next` prompt'una giren geçmiş metni. Adım adıyla birlikte
/// çalıştırılan tool çağrısını da gösterir.
pub fn format_history(history: &[StepRecord]) -> String {
    if history.is_empty() {
        return "(henüz hiçbir adım atılmadı)".to_string();
    }
    history
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let status = if r.success {
                "başarılı"
            } else {
                "başarısız"
            };
            let output_preview: String = r.output.chars().take(200).collect();
            let call = match &r.tool_call {
                Some(c) => format!(" [{} {:?}]", c.tool_name, c.arguments),
                None => String::new(),
            };
            if output_preview.is_empty() {
                format!("{}. {}{call} → {status} (çıktı yok)", i + 1, r.step_name)
            } else {
                format!(
                    "{}. {}{call} → {status}: {output_preview}",
                    i + 1,
                    r.step_name
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `plan_next`'in kararı: ya yeni bir adım, ya da "hedef tamamlandı".
#[derive(Debug, Clone)]
pub enum NextStepDecision {
    Step(AgentPlanStep),
    Done,
}

#[derive(Debug, Deserialize)]
struct NextStepResponse {
    // Bazı modeller adım yanıtında "done" alanını hiç yazmaz → varsayılan false.
    #[serde(default)]
    done: bool,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    retryable: bool,
    #[serde(default)]
    tool_name: Option<String>,
    // Sayı/bool gibi string olmayan argümanlar da kabul edilir (metne çevrilir).
    #[serde(default)]
    arguments: Vec<serde_json::Value>,
}

/// Model yanıtından JSON nesnesini ayıklar: ```json çitleri ya da önünde /
/// sonunda açıklama metni olsa bile ilk '{' ile son '}' arasını alır.
pub(crate) fn extract_json_object(raw: &str) -> &str {
    let t = raw.trim();
    match (t.find('{'), t.rfind('}')) {
        (Some(a), Some(b)) if b > a => &t[a..=b],
        _ => t,
    }
}

fn arg_to_string(v: serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    }
}

/// Tool listesini AI'ye gösterilecek satırlara çevirir: açıklaması
/// olan tool'lar "- isim: açıklama", olmayanlar sadece "- isim".
/// Hem `ai_plan` hem `plan_next` aynı formatı kullansın diye tek yerde.
fn describe_tools(tools: &[Arc<dyn AgentTool>]) -> String {
    tools
        .iter()
        .map(|t| {
            let description = t.description();
            if description.is_empty() {
                format!("- {}", t.name())
            } else {
                format!("- {}: {}", t.name(), description)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
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
        if let Some(router) = router
            && let Some(plan) = Self::ai_plan(objective.clone(), &router, tools).await
        {
            return plan;
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
                describe_tools(tools)
            )
        };

        let prompt =
            format!("Objective: {objective}\nCreate a concise execution plan with 2-5 steps.");

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

    /// V10 Sprint 7 (Autonomous Agent Loop): önceki adımın SONUCUNA
    /// bakarak tek bir sonraki adımı üretir. `plan()`'ın aksine tüm
    /// planı bir kerede vermez — her çağrı, geçmişteki gözlemlere göre
    /// yeniden düşünür. Sadece gerçek bir AI provider varken anlamlı;
    /// yoksa None döner (çağıran taraf sabit fallback_plan'a düşer —
    /// otonom döngü, uyarlanabilir bir planlayıcı olmadan zaten yapılamaz).
    pub async fn plan_next(
        objective: &str,
        history: &[StepRecord],
        router: &Arc<ProviderRouter>,
        tools: &[Arc<dyn AgentTool>],
    ) -> Option<NextStepDecision> {
        Self::plan_next_detailed(objective, history, router, tools)
            .await
            .ok()
    }

    /// `plan_next` ile aynı, ama başarısızlıkta SEBEBİ (kullanıcıya gösterilebilir
    /// kısa Türkçe metin) döner — "AI yanıt vermedi" belirsizliğini giderir.
    pub async fn plan_next_detailed(
        objective: &str,
        history: &[StepRecord],
        router: &Arc<ProviderRouter>,
        tools: &[Arc<dyn AgentTool>],
    ) -> Result<NextStepDecision, String> {
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name()).collect();

        let history_text = format_history(history);

        let tools_section = if tool_names.is_empty() {
            "No tools are available right now — never set \"tool_name\".".to_string()
        } else {
            format!(
                "Available tools (use ONLY these exact names in \"tool_name\"):\n{}",
                describe_tools(tools)
            )
        };

        let system = format!(
            r#"You are an AetherOS autonomous agent. Given an objective and the steps
taken so far (with their results), decide the SINGLE next step — or
say the objective is already complete.

Output ONLY valid JSON, no explanation, in exactly one of these two forms:
{{"done": true}}
{{"done": false, "name": "step_name", "retryable": true, "tool_name": "...", "arguments": ["..."]}}

{tools_section}

Only include "tool_name" if this specific step should invoke that tool.

Rules:
- NEVER repeat a step that already succeeded with the same tool and
  arguments. If the steps so far already satisfy the objective, answer
  {{"done": true}}. An empty result ("çıktı yok") from a successful command
  is NORMAL (e.g. touch, mkdir) — it means it worked.
- Tool "arguments" is a JSON array of strings; follow each tool's description
  for the exact order.
- For the terminal tool, arguments[0] is the program name only and every
  other word is its own element.
- For workspace_* tools every path is RELATIVE to the workspace root
  (e.g. "notes/a.txt"; "." is the root). NEVER use absolute paths or "..".
- If a step was denied or failed, do NOT retry the same call; try a
  different approach, or answer done: true if the objective cannot be met."#
        );

        let prompt = format!(
            "Objective: {objective}\n\nSteps so far:\n{history_text}\n\nWhat is the next step?"
        );

        let request = InferenceRequest::new(prompt, 512)
            .with_system(system)
            .with_temperature(0.3);

        let response = router
            .infer_active(request)
            .await
            .map_err(|e| format!("AI isteği başarısız: {e}"))?;

        debug!(
            tokens = response.tokens_used,
            "AI plan_next response received"
        );

        let cleaned = extract_json_object(&response.output);

        let parsed: NextStepResponse = serde_json::from_str(cleaned).map_err(|_| {
            let preview: String = response.output.trim().chars().take(80).collect();
            format!("AI yanıtı geçerli JSON değil: '{preview}'")
        })?;

        if parsed.done {
            return Ok(NextStepDecision::Done);
        }

        let name = parsed
            .name
            .filter(|n| !n.trim().is_empty())
            .ok_or_else(|| "AI yanıtında adım adı ('name') yok".to_string())?;
        let arguments: Vec<String> = parsed.arguments.into_iter().map(arg_to_string).collect();

        // Aynı güvenlik ilkesi: AI'nin ürettiği tool_name'e körü körüne
        // güvenilmiyor, sadece gerçek tool listesindeyse ToolCall'a çevrilir.
        let tool_call = parsed.tool_name.and_then(|tn| {
            if tool_names.contains(&tn.as_str()) {
                Some(ToolCall {
                    tool_name: tn,
                    arguments,
                })
            } else {
                tracing::warn!(
                    hallucinated_tool = %tn,
                    "plan_next: AI, tool listesinde olmayan bir tool ismi üretti — göz ardı edildi"
                );
                None
            }
        });

        Ok(NextStepDecision::Step(AgentPlanStep {
            id: Uuid::new_v4(),
            name,
            retryable: parsed.retryable,
            tool_call,
        }))
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
