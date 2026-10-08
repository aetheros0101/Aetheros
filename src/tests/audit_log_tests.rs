// ============================================================
// src/tests/audit_log_tests.rs
//
// V10 Sprint 6: AuditLog'un kendi davranışı (record/list/filter/kapasite)
// zaten logging/audit.rs'in kendi testlerinde kanıtlanıyor. Bu dosya
// FARKLI bir şeyi test ediyor: AgentRuntime.run_steps() GERÇEKTEN o
// AuditLog'a yazıyor mu — yani bağlantı gerçek mi, yoksa sadece
// bağlanabilir bir alan mı?
// ============================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use uuid::Uuid;

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::plans::{AgentPlanStep, ToolCall};
use crate::agents::runtime::AgentRuntime;
use crate::logging::audit::{AuditEventKind, AuditLog};
use crate::security::capability_engine::CapabilityEngine;
use crate::types::agent_tool::AgentTool;

struct EchoTool {
    invoked: Arc<AtomicBool>,
}

#[async_trait]
impl AgentTool for EchoTool {
    fn name(&self) -> &'static str {
        "echo_tool"
    }
    async fn invoke(&self, _arguments: Vec<String>) -> Result<String, String> {
        self.invoked.store(true, Ordering::SeqCst);
        Ok("ok".to_string())
    }
}

fn test_budget() -> AgentExecutionBudget {
    AgentExecutionBudget {
        max_tokens: 10_000,
        max_steps: 10,
        max_runtime_seconds: 30,
    }
}

#[tokio::test]
async fn successful_run_writes_governor_allow_tool_invoked_and_completed() {
    let invoked = Arc::new(AtomicBool::new(false));
    let tool: Arc<dyn AgentTool> = Arc::new(EchoTool {
        invoked: invoked.clone(),
    });
    let context = AgentContext {
        agent_id: Uuid::new_v4(),
        execution_id: Uuid::new_v4(),
        workflow_id: None,
    };
    let audit_log = Arc::new(AuditLog::new(100));

    let mut runtime = AgentRuntime::new(
        test_budget(),
        vec![tool],
        None,
        None,
        None,
        None,
        Some(audit_log.clone()),
    );

    let steps = vec![AgentPlanStep {
        id: Uuid::new_v4(),
        name: "step1".to_string(),
        retryable: false,
        tool_call: Some(ToolCall {
            tool_name: "echo_tool".to_string(),
            arguments: vec![],
        }),
    }];

    let outcome = runtime
        .run_steps(&context, &steps, 0, 0, "test objective")
        .await
        .expect("hatasız tamamlanmalı");

    assert!(invoked.load(Ordering::SeqCst));
    assert_eq!(outcome, crate::agents::runtime::AgentOutcome::Completed);

    let events = audit_log.list_for_execution(context.execution_id);
    assert!(
        events.iter().any(|e| matches!(
            &e.kind,
            AuditEventKind::GovernorDecision { decision, .. } if decision == "allow"
        )),
        "allow kararı denetime yazılmalı: {events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(&e.kind, AuditEventKind::ToolInvoked { success: true, .. })),
        "başarılı tool çağrısı denetime yazılmalı: {events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(&e.kind, AuditEventKind::ExecutionCompleted)),
        "tamamlanma denetime yazılmalı: {events:?}"
    );
}

#[tokio::test]
async fn denied_call_writes_governor_deny_and_execution_failed() {
    let tool: Arc<dyn AgentTool> = Arc::new(EchoTool {
        invoked: Arc::new(AtomicBool::new(false)),
    });
    let context = AgentContext {
        agent_id: Uuid::new_v4(),
        execution_id: Uuid::new_v4(),
        workflow_id: None,
    };
    let audit_log = Arc::new(AuditLog::new(100));

    let mut runtime = AgentRuntime::new(
        test_budget(),
        vec![tool],
        None,
        Some(Arc::new(CapabilityEngine::new())),
        None,
        None,
        Some(audit_log.clone()),
    );

    // Governor-bazlı deny senaryosu risk_policy_tests.rs'te zaten
    // kapsanıyor. Burada doğrulanan farklı bir şey: bilinmeyen bir
    // tool_name yüzünden invoke_tool_call'ın ürettiği hata, adım
    // retryable=false olduğunda GERÇEKTEN ExecutionFailed olarak
    // denetime yazılıyor mu.
    let steps = vec![AgentPlanStep {
        id: Uuid::new_v4(),
        name: "step1".to_string(),
        retryable: false,
        tool_call: Some(ToolCall {
            tool_name: "unknown_tool".to_string(),
            arguments: vec![],
        }),
    }];

    let result = runtime.run_steps(&context, &steps, 0, 0, "test").await;
    assert!(result.is_err());

    let events = audit_log.list_for_execution(context.execution_id);
    assert!(
        events
            .iter()
            .any(|e| matches!(&e.kind, AuditEventKind::ExecutionFailed { .. })),
        "bilinmeyen tool hatası ExecutionFailed olarak yazılmalı: {events:?}"
    );
}

#[tokio::test]
async fn audit_records_arguments_masked_and_output_summary() {
    let tool: Arc<dyn AgentTool> = Arc::new(EchoTool {
        invoked: Arc::new(AtomicBool::new(false)),
    });
    let context = AgentContext {
        agent_id: Uuid::new_v4(),
        execution_id: Uuid::new_v4(),
        workflow_id: None,
    };
    let audit_log = Arc::new(AuditLog::new(100));

    let mut runtime = AgentRuntime::new(
        test_budget(),
        vec![tool],
        None,
        None,
        None,
        None,
        Some(audit_log.clone()),
    );

    let steps = vec![AgentPlanStep {
        id: Uuid::new_v4(),
        name: "step1".to_string(),
        retryable: false,
        tool_call: Some(ToolCall {
            tool_name: "echo_tool".to_string(),
            arguments: vec![
                "merhaba".to_string(),
                "--password".to_string(),
                "s3cret".to_string(),
            ],
        }),
    }];

    runtime
        .run_steps(&context, &steps, 0, 0, "test")
        .await
        .expect("hatasız tamamlanmalı");

    let events = audit_log.list_for_execution(context.execution_id);

    // Governor kararı argümanlarıyla birlikte, sır maskeli yazılmalı.
    let gov_args = events
        .iter()
        .find_map(|e| match &e.kind {
            AuditEventKind::GovernorDecision { arguments, .. } => Some(arguments.clone()),
            _ => None,
        })
        .expect("GovernorDecision olayı yok");
    assert_eq!(gov_args, vec!["merhaba", "--password", "[REDACTED]"]);

    // Tool çağrısı: argümanlar + çıktı özeti ("ok" → 2 bayt).
    let (inv_args, out) = events
        .iter()
        .find_map(|e| match &e.kind {
            AuditEventKind::ToolInvoked {
                arguments,
                output,
                success: true,
                ..
            } => Some((arguments.clone(), output.clone())),
            _ => None,
        })
        .expect("başarılı ToolInvoked olayı yok");
    assert_eq!(inv_args, vec!["merhaba", "--password", "[REDACTED]"]);
    let out = out.expect("çıktı özeti yazılmalı");
    assert_eq!(out.bytes, 2);
    assert_eq!(out.sha256.len(), 64);
    assert_eq!(out.preview, "ok");

    // Ham sır hiçbir olayda görünmemeli.
    let dump = format!("{events:?}");
    assert!(!dump.contains("s3cret"), "sır audit'e sızdı: {dump}");
}
