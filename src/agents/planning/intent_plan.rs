//! Niyet (intent) tabanlı planlama.
//!
//! `aetheros-intent` boru hattını (niyet → gereksinim → doğrulama → görev
//! grafı) çalıştırır ve topolojik sıradaki görevleri `AgentPlan` adımlarına
//! çevirir. Deterministiktir (aynı graf → aynı sıra) ve `Constraints`
//! (alan listeleri, derinlik, paralellik) burada uygulanır.
//!
//! Güvenlik notu: görevlerin `capabilities` alanı yalnızca *ipucudur*; adımlar
//! `tool_call` taşımaz. Hiçbir araç, Governor/Capability/Approval hattını
//! atlayarak çağrılamaz.

use aetheros_intent::{Constraints, IntentError, IntentPipeline};

use crate::agents::plans::{AgentPlan, AgentPlanStep};

/// Hedefi niyet boru hattından geçirip plana çevirir.
///
/// Adım kimliği, derlenmiş görevin kimliğidir (izlenebilirlik: plan adımı ↔
/// görev düğümü ↔ kaynak gereksinim).
pub fn plan_from_intent(
    objective: &str,
    constraints: Constraints,
) -> Result<AgentPlan, IntentError> {
    let result = IntentPipeline::new(constraints).process(objective)?;
    let tasks = result.graph.to_compiled_tasks()?;

    let planned_steps = tasks
        .into_iter()
        .map(|t| AgentPlanStep {
            id: t.id,
            name: t.title,
            retryable: true,
            tool_call: None,
        })
        .collect::<Vec<_>>();

    if planned_steps.is_empty() {
        return Err(IntentError::Compile("empty task graph".into()));
    }

    Ok(AgentPlan {
        id: result.intent.id,
        objective: objective.to_string(),
        planned_steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn readme_objective_becomes_a_small_toolless_plan() {
        let plan = plan_from_intent("README dosyasını düzelt", Constraints::default()).unwrap();
        assert!(plan.planned_steps.len() >= 2);
        assert!(plan.planned_steps[0].name.starts_with("Analyze"));
        assert!(plan.planned_steps.iter().all(|s| s.tool_call.is_none()));
        let ids: HashSet<_> = plan.planned_steps.iter().map(|s| s.id).collect();
        assert_eq!(ids.len(), plan.planned_steps.len(), "adım id'leri benzersiz");
    }

    #[test]
    fn step_order_is_stable_across_runs_in_shape() {
        let names = |p: &AgentPlan| p.planned_steps.iter().map(|s| s.name.clone()).collect::<Vec<_>>();
        let a = plan_from_intent("API endpoint ekle", Constraints::default()).unwrap();
        let b = plan_from_intent("API endpoint ekle", Constraints::default()).unwrap();
        assert_eq!(names(&a), names(&b));
    }

    #[test]
    fn ambiguous_objective_is_an_error_not_a_guess() {
        assert!(plan_from_intent("yap", Constraints::default()).is_err());
    }

    #[test]
    fn denied_domain_is_respected() {
        let c = Constraints {
            denied_domains: vec!["docs".into()],
            ..Constraints::default()
        };
        let r = plan_from_intent("README dosyasını düzelt", c);
        assert!(matches!(r, Err(IntentError::Constraint(_))), "{r:?}");
    }
}
