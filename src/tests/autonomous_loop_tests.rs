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
