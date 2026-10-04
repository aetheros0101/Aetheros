// ============================================================
// src/tests/autonomous_loop_tests.rs
//
// V10 Sprint 7: Autonomous Agent Loop (guarded).
//
// Gerçek bir AI provider olmadan plan_next()'in hallucination-filtreleme
// veya çok-adımlı yeniden planlama davranışını test edemeyiz (Sprint 2'de
// ai_plan() için de aynı sebeple yapılmadı — sahte bir ProviderRouter
// kurmak bu testi ya kırılgan ya da anlamsız kılardı). Burada test
// edilen: (1) aktif provider yokken plan_next'in gerçekten None
// döndüğü, (2) ai_router Some ama aktif provider yokken otonom
// döngünün sabit fallback plana GÜVENLE düştüğü — yani "ai_router var"
// demek "hep otonom, hiç güvenlik ağı yok" demek değil.
// ============================================================

use std::sync::Arc;

use uuid::Uuid;

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::executor::AgentExecutor;
use crate::agents::planner::{AgentPlanner, StepRecord};
use crate::agents::runtime::AgentOutcome;
use crate::ai::routing::router::ProviderRouter;

fn test_budget() -> AgentExecutionBudget {
    AgentExecutionBudget {
        max_tokens: 10_000,
        max_steps: 10,
        max_runtime_seconds: 30,
    }
}

#[tokio::test]
async fn plan_next_returns_none_without_an_active_provider() {
    let router = Arc::new(ProviderRouter::new());
    let history: Vec<StepRecord> = vec![];

    let decision = AgentPlanner::plan_next("bir hedef", &history, &router, &[]).await;

    assert!(
        decision.is_none(),
        "aktif provider yokken plan_next None dönmeli, sessizce hayali bir adım üretmemeli"
    );
}

#[tokio::test]
async fn autonomous_loop_falls_back_to_fixed_plan_without_an_active_provider() {
    // ai_router Some (bağlı) AMA aktif provider yok — tam olarak
    // "bridge her zaman Some(ai_router) geçer, provider'ın kendisi
    // aktif olmayabilir" senaryosu (gerçek mobil bridge'in yaptığı gibi).
    let router = Arc::new(ProviderRouter::new());
    let context = AgentContext {
        agent_id: Uuid::new_v4(),
        execution_id: Uuid::new_v4(),
        workflow_id: None,
    };

    let outcome = AgentExecutor::execute(
        context,
        "test objective".to_string(),
        test_budget(),
        vec![],
        Some(router),
        None,
        None,
        None,
        None,
    )
    .await
    .expect("aktif provider olmasa bile execution hatasız tamamlanmalı");

    assert_eq!(
        outcome,
        AgentOutcome::Completed,
        "otonom döngü ilk adımda başarısız olunca sabit fallback plana düşüp tamamlamalı"
    );
}

// ── Tekrar koruması + planlayıcı geçmişi (saf mantık, provider gerekmez) ──

use crate::agents::plans::{AgentPlanStep, ToolCall};
use crate::agents::planner::{format_history, is_repeat_of_last};

fn call(tool: &str, args: &[&str]) -> ToolCall {
    ToolCall {
        tool_name: tool.to_string(),
        arguments: args.iter().map(|s| s.to_string()).collect(),
    }
}

fn step_with(c: Option<ToolCall>) -> AgentPlanStep {
    AgentPlanStep {
        id: Uuid::new_v4(),
        name: "adim".to_string(),
        retryable: false,
        tool_call: c,
    }
}

fn record(c: Option<ToolCall>, success: bool, output: &str) -> StepRecord {
    StepRecord {
        step_name: "adim".to_string(),
        success,
        output: output.to_string(),
        tool_call: c,
    }
}

#[test]
fn identical_consecutive_successful_call_is_a_repeat() {
    let history = vec![record(Some(call("terminal", &["ls"])), true, "deneme.txt")];
    assert!(is_repeat_of_last(&history, &step_with(Some(call("terminal", &["ls"])))));
}

#[test]
fn different_arguments_or_tool_are_not_a_repeat() {
    let history = vec![record(Some(call("terminal", &["ls"])), true, "x")];
    assert!(!is_repeat_of_last(&history, &step_with(Some(call("terminal", &["ls", "-a"])))));
    assert!(!is_repeat_of_last(&history, &step_with(Some(call("baska", &["ls"])))));
}

#[test]
fn failed_previous_call_may_be_retried() {
    let history = vec![record(Some(call("terminal", &["ls"])), false, "hata")];
    assert!(!is_repeat_of_last(&history, &step_with(Some(call("terminal", &["ls"])))));
}

#[test]
fn only_the_immediately_preceding_step_counts() {
    // ls → touch → ls meşru bir doğrulama akışıdır.
    let history = vec![
        record(Some(call("terminal", &["ls"])), true, "a"),
        record(Some(call("terminal", &["touch", "x"])), true, ""),
    ];
    assert!(!is_repeat_of_last(&history, &step_with(Some(call("terminal", &["ls"])))));
}

#[test]
fn empty_history_and_tool_less_steps_are_never_repeats() {
    assert!(!is_repeat_of_last(&[], &step_with(Some(call("terminal", &["ls"])))));
    let history = vec![record(None, true, "")];
    assert!(!is_repeat_of_last(&history, &step_with(None)));
}

#[test]
fn history_text_shows_the_executed_command_and_marks_empty_output() {
    let text = format_history(&[
        record(Some(call("terminal", &["touch", "deneme.txt"])), true, ""),
        record(Some(call("terminal", &["ls"])), true, "deneme.txt"),
    ]);
    assert!(text.contains(r#"[terminal ["touch", "deneme.txt"]]"#), "{text}");
    assert!(text.contains("çıktı yok"), "{text}");
    assert!(text.contains("başarılı: deneme.txt"), "{text}");
    assert_eq!(format_history(&[]), "(henüz hiçbir adım atılmadı)");
}
