// ============================================================
// src/tests/approval_engine_tests.rs
//
// V10 Sprint 5: Approval Engine (durdur & sonra devam ettir).
//
// AgentPlanner'ın fallback_plan'ı (AI'siz ortamda) hiçbir adıma
// tool_call eklemediği için (bkz. Sprint 2), run()'ı uçtan uca (planner
// dahil) tetikleyerek bu davranışı test etmek mümkün değil. Bunun
// yerine run_steps()'i (pub(crate)) doğrudan, elle kurulmuş bir adım
// listesiyle çağırıyoruz — deterministik ve AI'ye bağımlı değil.
// ============================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use uuid::Uuid;

use crate::agents::approval::ApprovalStore;
use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::plans::{AgentPlanStep, ToolCall};
use crate::agents::runtime::{AgentOutcome, AgentRuntime, HighRiskPolicy};
use crate::security::risk_engine::RiskEngine;
use crate::types::agent_tool::{AgentTool, RiskLevel};

struct RiskyTool {
    invoked: Arc<AtomicBool>,
}

#[async_trait]
impl AgentTool for RiskyTool {
    fn name(&self) -> &'static str {
        "risky_tool"
    }
    fn risk_level(&self) -> RiskLevel {
        RiskLevel::High
    }
    async fn invoke(&self, _arguments: Vec<String>) -> Result<String, String> {
        self.invoked.store(true, Ordering::SeqCst);
        Ok("ran".to_string())
    }
}

fn test_budget() -> AgentExecutionBudget {
    AgentExecutionBudget {
        max_tokens: 10_000,
        max_steps: 10,
        max_runtime_seconds: 30,
    }
}

fn test_context() -> AgentContext {
    AgentContext {
        agent_id: Uuid::new_v4(),
        execution_id: Uuid::new_v4(),
        workflow_id: None,
    }
}

fn risky_step() -> AgentPlanStep {
    AgentPlanStep {
        id: Uuid::new_v4(),
        name: "do risky thing".to_string(),
        retryable: false,
        tool_call: Some(ToolCall {
            tool_name: "risky_tool".to_string(),
            arguments: vec!["arg1".to_string()],
        }),
    }
}

fn harmless_step() -> AgentPlanStep {
    AgentPlanStep {
        id: Uuid::new_v4(),
        name: "finalize".to_string(),
        retryable: false,
        tool_call: None,
    }
}

#[tokio::test]
async fn require_approval_pauses_before_ever_invoking_the_tool() {
    let invoked = Arc::new(AtomicBool::new(false));
    let tool: Arc<dyn AgentTool> = Arc::new(RiskyTool {
        invoked: invoked.clone(),
    });
    let context = test_context();
    let approval_store = Arc::new(ApprovalStore::new());

    let mut runtime = AgentRuntime::new(
        test_budget(),
        vec![tool],
        None,
        None,
        Some(Arc::new(RiskEngine::new())),
        Some(approval_store.clone()),
        None,
    );
    runtime.set_high_risk_policy(HighRiskPolicy::RequireApproval);

    let steps = vec![risky_step(), harmless_step()];
    let outcome = runtime
        .run_steps(&context, &steps, 0, 0, "test objective")
        .await
        .expect("run_steps hata dönmemeli, duraklamalı");

    // 1. Tool'a HİÇ dokunulmadı.
    assert!(
        !invoked.load(Ordering::SeqCst),
        "RequiresApproval, tool invoke edilmeden ÖNCE durmalı"
    );

    // 2. Doğru outcome döndü.
    let approval_id = match outcome {
        AgentOutcome::PendingApproval { approval_id } => approval_id,
        other => panic!("PendingApproval bekleniyordu, gelen: {other:?}"),
    };

    // 3. ApprovalStore'a gerçekten yazıldı, ve kalan adım doğru.
    let pending = approval_store
        .get(&approval_id)
        .expect("PendingApproval store'da olmalı");
    assert_eq!(pending.tool_call.tool_name, "risky_tool");
    assert_eq!(pending.tool_call.arguments, vec!["arg1".to_string()]);
    assert_eq!(pending.remaining_steps.len(), 1);
    assert_eq!(pending.remaining_steps[0].name, "finalize");
}

#[tokio::test]
async fn resume_after_approval_actually_invokes_the_tool_and_continues() {
    let invoked = Arc::new(AtomicBool::new(false));
    let tool: Arc<dyn AgentTool> = Arc::new(RiskyTool {
        invoked: invoked.clone(),
    });
    let context = test_context();
    let approval_store = Arc::new(ApprovalStore::new());

    let mut runtime = AgentRuntime::new(
        test_budget(),
        vec![tool.clone()],
        None,
        None,
        Some(Arc::new(RiskEngine::new())),
        Some(approval_store.clone()),
        None,
    );
    runtime.set_high_risk_policy(HighRiskPolicy::RequireApproval);

    let steps = vec![risky_step(), harmless_step()];
    let outcome = runtime
        .run_steps(&context, &steps, 0, 0, "test objective")
        .await
        .unwrap();
    let approval_id = match outcome {
        AgentOutcome::PendingApproval { approval_id } => approval_id,
        other => panic!("PendingApproval bekleniyordu: {other:?}"),
    };

    // Kullanıcı onayladı: kaydı al (bridge'in respond_to_approval'ının
    // yaptığı gibi) ve resume et.
    let pending = approval_store.take(&approval_id).unwrap();

    let outcome = AgentRuntime::resume(
        pending,
        vec![tool],
        None,
        None,
        Some(Arc::new(RiskEngine::new())),
        Some(approval_store),
        None,
    )
    .await
    .expect("resume hatasız tamamlanmalı");

    assert!(
        invoked.load(Ordering::SeqCst),
        "onaylanınca tool GERÇEKTEN çalışmalı"
    );
    assert_eq!(outcome, AgentOutcome::Completed);
}

#[tokio::test]
async fn block_policy_never_pauses_it_denies_immediately() {
    // Karşılaştırma: Block politikasında (varsayılan, Sprint 3'ten beri)
    // hiç duraklama olmaz — doğrudan reddedilir, ApprovalStore hiç
    // kullanılmaz.
    let invoked = Arc::new(AtomicBool::new(false));
    let tool: Arc<dyn AgentTool> = Arc::new(RiskyTool {
        invoked: invoked.clone(),
    });
    let context = test_context();
    let approval_store = Arc::new(ApprovalStore::new());

    // high_risk_policy hiç değiştirilmedi — varsayılan Block.
    let mut runtime = AgentRuntime::new(
        test_budget(),
        vec![tool],
        None,
        None,
        Some(Arc::new(RiskEngine::new())),
        Some(approval_store.clone()),
        None,
    );

    let steps = vec![risky_step()];
    let result = runtime.run_steps(&context, &steps, 0, 0, "test").await;

    assert!(
        result.is_err(),
        "Block politikasında adım hata ile bitmeli (retryable=false)"
    );
    assert!(!invoked.load(Ordering::SeqCst));
    assert!(
        approval_store.list().is_empty(),
        "Block yolunda hiç PendingApproval oluşmamalı"
    );
}
