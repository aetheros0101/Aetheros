// ============================================================
// src/tests/command_policy_tests.rs
//
// B4 çıkış kriteri, runtime üzerinden uçtan uca:
//   ["sh","-c","…"]  → kesin ret (onay istenmez, çalışmaz)
//   ["git","status"] → onaysız çalışır
//   ["git","commit"] → duraklar, onay bekler
//   onay sonrası resume bile Deny'ı aşamaz
//
// Gerçek süreç başlatmamak için, gerçek CommandPolicy'yi kullanan ama
// invoke'da yalnızca kaydeden bir sahte "terminal" tool'u kullanılır.
// ============================================================

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::agents::approval::ApprovalStore;
use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::plans::{AgentPlanStep, ToolCall};
use crate::agents::runtime::{AgentOutcome, AgentRuntime};
use crate::security::command_policy::CommandPolicy;
use crate::security::risk_engine::RiskEngine;
use crate::types::agent_tool::{AgentTool, CallVerdict, RiskLevel};

struct ProbeTerminal {
    policy: CommandPolicy,
    invoked: Arc<AtomicUsize>,
}

#[async_trait]
impl AgentTool for ProbeTerminal {
    fn name(&self) -> &'static str {
        "terminal"
    }
    fn risk_level(&self) -> RiskLevel {
        RiskLevel::High
    }
    fn assess_call(&self, arguments: &[String]) -> Option<CallVerdict> {
        Some(self.policy.assess(arguments))
    }
    async fn invoke(&self, _arguments: Vec<String>) -> Result<String, String> {
        self.invoked.fetch_add(1, Ordering::SeqCst);
        Ok("ok".into())
    }
}

fn budget() -> AgentExecutionBudget {
    AgentExecutionBudget {
        max_tokens: 10_000,
        max_steps: 10,
        max_runtime_seconds: 30,
    }
}

fn ctx() -> AgentContext {
    AgentContext {
        agent_id: Uuid::new_v4(),
        execution_id: Uuid::new_v4(),
        workflow_id: None,
    }
}

fn step(args: &[&str]) -> AgentPlanStep {
    AgentPlanStep {
        id: Uuid::new_v4(),
        name: "terminal adımı".into(),
        retryable: false,
        tool_call: Some(ToolCall {
            tool_name: "terminal".into(),
            arguments: args.iter().map(|s| s.to_string()).collect(),
        }),
    }
}

fn setup() -> (AgentRuntime, Arc<ApprovalStore>, Arc<AtomicUsize>, Arc<dyn AgentTool>) {
    let invoked = Arc::new(AtomicUsize::new(0));
    let tool: Arc<dyn AgentTool> = Arc::new(ProbeTerminal {
        policy: CommandPolicy::default_policy(),
        invoked: invoked.clone(),
    });
    let store = Arc::new(ApprovalStore::new());
    let runtime = AgentRuntime::new(
        budget(),
        vec![tool.clone()],
        None,
        None,
        Some(Arc::new(RiskEngine::new())),
        Some(store.clone()),
        None,
    );
    (runtime, store, invoked, tool)
}

#[tokio::test]
async fn git_status_runs_without_approval() {
    let (mut runtime, store, invoked, _) = setup();
    let outcome = runtime
        .run_steps(&ctx(), &[step(&["git", "status"])], 0, 0, "t")
        .await
        .expect("git status hatasız çalışmalı");
    assert_eq!(outcome, AgentOutcome::Completed);
    assert_eq!(invoked.load(Ordering::SeqCst), 1);
    assert!(store.list().is_empty(), "onay oluşmamalı");
}

#[tokio::test]
async fn shell_is_denied_without_pausing_or_running() {
    let (mut runtime, store, invoked, _) = setup();
    let result = runtime
        .run_steps(&ctx(), &[step(&["sh", "-c", "id"])], 0, 0, "t")
        .await;
    assert!(result.is_err(), "Deny adım hatasıyla bitmeli");
    assert_eq!(invoked.load(Ordering::SeqCst), 0, "çalışmamalı");
    assert!(store.list().is_empty(), "Deny onaya düşmemeli");
}

#[tokio::test]
async fn git_commit_pauses_for_approval_with_arguments_kept() {
    let (mut runtime, store, invoked, _) = setup();
    let outcome = runtime
        .run_steps(&ctx(), &[step(&["git", "commit", "-m", "x"])], 0, 0, "t")
        .await
        .unwrap();
    assert!(matches!(outcome, AgentOutcome::PendingApproval { .. }));
    assert_eq!(invoked.load(Ordering::SeqCst), 0);
    let pending = store.list();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].tool_call.arguments, vec!["git", "commit", "-m", "x"]);
}

#[tokio::test]
async fn approved_ask_resumes_and_runs() {
    let (mut runtime, store, invoked, tool) = setup();
    let outcome = runtime
        .run_steps(&ctx(), &[step(&["git", "commit", "-m", "x"])], 0, 0, "t")
        .await
        .unwrap();
    let id = match outcome {
        AgentOutcome::PendingApproval { approval_id } => approval_id,
        o => panic!("{o:?}"),
    };
    let pending = store.take(&id).unwrap();
    let out = AgentRuntime::resume(pending, vec![tool], None, None, None, None, None)
        .await
        .unwrap();
    assert_eq!(out, AgentOutcome::Completed);
    assert_eq!(invoked.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn resume_cannot_bypass_a_policy_deny() {
    // Eski/elle üretilmiş bir onay kaydı yasaklı bir komut taşıyor olsa bile
    // resume onu ÇALIŞTIRMAZ.
    let (mut runtime, store, invoked, tool) = setup();
    // Önce meşru bir Ask ile geçerli bir PendingApproval al, sonra komutu değiştir.
    let outcome = runtime
        .run_steps(&ctx(), &[step(&["git", "commit"])], 0, 0, "t")
        .await
        .unwrap();
    let id = match outcome {
        AgentOutcome::PendingApproval { approval_id } => approval_id,
        o => panic!("{o:?}"),
    };
    let mut pending = store.take(&id).unwrap();
    pending.tool_call.arguments = vec!["sh".into(), "-c".into(), "id".into()];

    let result = AgentRuntime::resume(pending, vec![tool], None, None, None, None, None).await;
    // Hata dönebilir ya da Completed (hata audit'e yazılır); önemli olan ÇALIŞMAMASI.
    let _ = result;
    assert_eq!(invoked.load(Ordering::SeqCst), 0, "Deny resume ile aşılmamalı");
}

// ── B7: resume capability'yi atlamaz ─────────────────────────

struct CapProbe {
    invoked: Arc<AtomicUsize>,
}

#[async_trait]
impl AgentTool for CapProbe {
    fn name(&self) -> &'static str {
        "cap_probe"
    }
    fn required_capability(&self) -> Option<crate::agents::capabilities::AgentCapability> {
        Some(crate::agents::capabilities::AgentCapability::TerminalExecution)
    }
    fn risk_level(&self) -> RiskLevel {
        RiskLevel::High
    }
    async fn invoke(&self, _arguments: Vec<String>) -> Result<String, String> {
        self.invoked.fetch_add(1, Ordering::SeqCst);
        Ok("ok".into())
    }
}

fn cap_pending() -> crate::agents::approval::PendingApproval {
    crate::agents::approval::PendingApproval {
        id: Uuid::new_v4(),
        context: ctx(),
        objective: "t".into(),
        tool_call: ToolCall {
            tool_name: "cap_probe".into(),
            arguments: vec!["x".into()],
        },
        reason: "test".into(),
        remaining_steps: vec![],
        budget: budget(),
        created_at: chrono::Utc::now(),
        autonomous: false,
        history: vec![],
    }
}

#[tokio::test]
async fn resume_respects_revoked_capability() {
    // Onay verildiği anda agent'ın capability'si yok (geri alınmış / hiç yok):
    // onay, capability reddini AŞAMAZ.
    let invoked = Arc::new(AtomicUsize::new(0));
    let tool: Arc<dyn AgentTool> = Arc::new(CapProbe { invoked: invoked.clone() });
    let cap_engine = Arc::new(crate::security::capability_engine::CapabilityEngine::new());

    let _ = AgentRuntime::resume(
        cap_pending(),
        vec![tool],
        None,
        Some(cap_engine),
        Some(Arc::new(RiskEngine::new())),
        None,
        None,
    )
    .await;

    assert_eq!(invoked.load(Ordering::SeqCst), 0, "capability yokken resume çalıştırmamalı");
}

#[tokio::test]
async fn resume_fail_closed_without_capability_engine() {
    // Motor hiç bağlı değilse (None) capability isteyen tool resume'da da çalışmaz.
    let invoked = Arc::new(AtomicUsize::new(0));
    let tool: Arc<dyn AgentTool> = Arc::new(CapProbe { invoked: invoked.clone() });

    let _ = AgentRuntime::resume(cap_pending(), vec![tool], None, None, None, None, None).await;

    assert_eq!(invoked.load(Ordering::SeqCst), 0);
}
