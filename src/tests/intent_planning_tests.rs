//! `AgentPlanner::plan_with_intent`: AI → intent → fallback sırası.

use aetheros_intent::Constraints;

use crate::agents::planning::AgentPlanner;

#[tokio::test]
async fn without_router_a_clear_objective_uses_the_intent_plan() {
    let plan = AgentPlanner::plan_with_intent(
        "README dosyasını düzelt".to_string(),
        None,
        &[],
        Constraints::default(),
    )
    .await;
    assert!(
        !plan.planned_steps.iter().any(|s| s.name == "initialize"),
        "sabit fallback değil, intent planı beklenir"
    );
    assert!(plan.planned_steps.iter().all(|s| s.tool_call.is_none()));
}

#[tokio::test]
async fn ambiguous_objective_falls_back_to_the_fixed_plan() {
    let plan =
        AgentPlanner::plan_with_intent("yap".to_string(), None, &[], Constraints::default()).await;
    assert_eq!(plan.planned_steps.first().map(|s| s.name.as_str()), Some("initialize"));
}

#[tokio::test]
async fn existing_plan_entry_point_is_unchanged() {
    let plan = AgentPlanner::plan("README dosyasını düzelt".to_string(), None, &[]).await;
    assert_eq!(plan.planned_steps.len(), 3);
    assert_eq!(plan.planned_steps[0].name, "initialize");
}
