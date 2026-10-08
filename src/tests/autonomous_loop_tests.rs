// ============================================================
// src/tests/autonomous_loop_tests.rs
//
// V10 Sprint 7: Autonomous Agent Loop (guarded).
//
// Test edilenler: (1) aktif provider yokken plan_next None döner, (2) aktif
// provider yokken otonom döngü DÜRÜSTÇE başarısız olur (boş plan + sahte
// "completed" yok), (3) senaryolu sahte provider ile: onaydan sonra döngü
// kaldığı yerden DEVAM eder (B6), hiçbir adım başarılı olmazsa başarısız
// sayılır.
// ============================================================

use std::sync::Arc;

use uuid::Uuid;

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::executor::AgentExecutor;
use crate::agents::planner::{AgentPlanner, NextStepDecision, StepRecord, extract_json_object};
use crate::agents::runtime::{AgentOutcome, AgentRuntime};
use crate::ai::routing::router::ProviderRouter;
use crate::errors::runtime::RuntimeError;
use crate::logging::audit::{AuditEventKind, AuditLog};

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
async fn autonomous_loop_fails_honestly_without_an_active_provider() {
    // ai_router Some (bağlı) AMA aktif provider yok — gerçek mobil bridge'in
    // durumu. Eskiden boş bir sabit plana düşüp "completed" deniyordu;
    // hiçbir şey yapılmadığı hâlde başarı görünüyordu.
    let router = Arc::new(ProviderRouter::new());
    let audit = Arc::new(AuditLog::new(100));
    let context = AgentContext {
        agent_id: Uuid::new_v4(),
        execution_id: Uuid::new_v4(),
        workflow_id: None,
    };
    let exec_id = context.execution_id;

    let result = AgentExecutor::execute(
        context,
        "test objective".to_string(),
        test_budget(),
        vec![],
        Some(router),
        None,
        None,
        None,
        Some(audit.clone()),
    )
    .await;

    match result {
        Err(RuntimeError::TaskExecutionFailed { message }) => {
            assert!(message.contains("AI sağlayıcı aktif değil"), "{message}");
        }
        other => panic!("başarısızlık bekleniyordu, gelen: {other:?}"),
    }
    let events = audit.list_for_execution(exec_id);
    assert!(
        events
            .iter()
            .any(|e| matches!(e.kind, AuditEventKind::ExecutionFailed { .. })),
        "ExecutionFailed denetime yazılmalı"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e.kind, AuditEventKind::ExecutionCompleted)),
        "başarısız execution 'tamamlandı' yazmamalı"
    );
}

// ── Tekrar koruması + planlayıcı geçmişi (saf mantık, provider gerekmez) ──

use crate::agents::planner::{format_history, is_repeat_of_last};
use crate::agents::plans::{AgentPlanStep, ToolCall};

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
    assert!(is_repeat_of_last(
        &history,
        &step_with(Some(call("terminal", &["ls"])))
    ));
}

#[test]
fn different_arguments_or_tool_are_not_a_repeat() {
    let history = vec![record(Some(call("terminal", &["ls"])), true, "x")];
    assert!(!is_repeat_of_last(
        &history,
        &step_with(Some(call("terminal", &["ls", "-a"])))
    ));
    assert!(!is_repeat_of_last(
        &history,
        &step_with(Some(call("baska", &["ls"])))
    ));
}

#[test]
fn failed_previous_call_may_be_retried() {
    let history = vec![record(Some(call("terminal", &["ls"])), false, "hata")];
    assert!(!is_repeat_of_last(
        &history,
        &step_with(Some(call("terminal", &["ls"])))
    ));
}

#[test]
fn only_the_immediately_preceding_step_counts() {
    // ls → touch → ls meşru bir doğrulama akışıdır.
    let history = vec![
        record(Some(call("terminal", &["ls"])), true, "a"),
        record(Some(call("terminal", &["touch", "x"])), true, ""),
    ];
    assert!(!is_repeat_of_last(
        &history,
        &step_with(Some(call("terminal", &["ls"])))
    ));
}

#[test]
fn empty_history_and_tool_less_steps_are_never_repeats() {
    assert!(!is_repeat_of_last(
        &[],
        &step_with(Some(call("terminal", &["ls"])))
    ));
    let history = vec![record(None, true, "")];
    assert!(!is_repeat_of_last(&history, &step_with(None)));
}

#[test]
fn history_text_shows_the_executed_command_and_marks_empty_output() {
    let text = format_history(&[
        record(Some(call("terminal", &["touch", "deneme.txt"])), true, ""),
        record(Some(call("terminal", &["ls"])), true, "deneme.txt"),
    ]);
    assert!(
        text.contains(r#"[terminal ["touch", "deneme.txt"]]"#),
        "{text}"
    );
    assert!(text.contains("çıktı yok"), "{text}");
    assert!(text.contains("başarılı: deneme.txt"), "{text}");
    assert_eq!(format_history(&[]), "(henüz hiçbir adım atılmadı)");
}

// ── Senaryolu sahte provider: onay sonrası devam (B6) ──

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;

use crate::agents::approval::ApprovalStore;
use crate::agents::capabilities::AgentCapability;
use crate::ai::errors::AiError;
use crate::ai::inference::request::InferenceRequest;
use crate::ai::inference::response::InferenceResponse;
use crate::ai::providers::provider::ModelProvider;
use crate::security::capability_engine::CapabilityEngine;
use crate::security::risk_engine::RiskEngine;
use crate::types::agent_tool::{AgentTool, RiskLevel};

/// Sıradaki hazır yanıtı döner; biterse `{"done": true}`. Aldığı istemleri saklar.
struct ScriptedProvider {
    replies: Mutex<VecDeque<String>>,
    prompts: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl ModelProvider for ScriptedProvider {
    fn provider_id(&self) -> &'static str {
        "scripted"
    }
    fn supports_streaming(&self) -> bool {
        false
    }
    async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse, AiError> {
        // Araç listesi system_prompt'ta gider; testler ikisini birlikte görsün.
        let seen = format!(
            "{}\n{}",
            request.system_prompt.clone().unwrap_or_default(),
            request.prompt
        );
        self.prompts.lock().unwrap().push(seen);
        let next = self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| r#"{"done": true}"#.to_string());
        Ok(InferenceResponse {
            output: next,
            tokens_used: 1,
        })
    }
}

fn scripted_router(replies: &[&str]) -> (Arc<ProviderRouter>, Arc<Mutex<Vec<String>>>) {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let router = Arc::new(ProviderRouter::new());
    router.register(Arc::new(ScriptedProvider {
        replies: Mutex::new(replies.iter().map(|s| s.to_string()).collect()),
        prompts: prompts.clone(),
    }));
    router.set_active("scripted").unwrap();
    (router, prompts)
}

/// Yüksek riskli (onay ister) ve düşük riskli iki araç; çağrı sayısını tutar.
struct CountTool {
    name: &'static str,
    risk: RiskLevel,
    calls: Arc<AtomicUsize>,
    fail: bool,
    cap: Option<AgentCapability>,
}

#[async_trait]
impl AgentTool for CountTool {
    fn name(&self) -> &'static str {
        self.name
    }
    fn risk_level(&self) -> RiskLevel {
        self.risk
    }
    fn required_capability(&self) -> Option<AgentCapability> {
        self.cap
    }
    async fn invoke(&self, _a: Vec<String>) -> Result<String, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            Err("bilerek hata".to_string())
        } else {
            Ok(format!("{} tamam", self.name))
        }
    }
}

fn ctx() -> AgentContext {
    AgentContext {
        agent_id: Uuid::new_v4(),
        execution_id: Uuid::new_v4(),
        workflow_id: None,
    }
}

const STEP_RISKY: &str = r#"{"done": false, "name": "riskli", "retryable": false, "tool_name": "risky", "arguments": ["a"]}"#;
const STEP_SECOND_RETRYABLE: &str = r#"{"done": false, "name": "ikinci", "retryable": true, "tool_name": "second", "arguments": ["b"]}"#;
const STEP_SECOND: &str = r#"{"done": false, "name": "ikinci", "retryable": false, "tool_name": "second", "arguments": ["b"]}"#;

#[tokio::test]
async fn autonomous_loop_continues_after_an_approved_step() {
    let risky_calls = Arc::new(AtomicUsize::new(0));
    let second_calls = Arc::new(AtomicUsize::new(0));
    let mk_tools = || -> Vec<Arc<dyn AgentTool>> {
        vec![
            Arc::new(CountTool {
                name: "risky",
                risk: RiskLevel::High,
                calls: risky_calls.clone(),
                fail: false,
                cap: None,
            }),
            Arc::new(CountTool {
                name: "second",
                risk: RiskLevel::Low,
                calls: second_calls.clone(),
                fail: false,
                cap: None,
            }),
        ]
    };
    let (router, prompts) = scripted_router(&[STEP_RISKY, STEP_SECOND, r#"{"done": true}"#]);
    let store = Arc::new(ApprovalStore::new());
    let context = ctx();

    // 1) İlk çalıştırma: riskli adımda duraklar, araç ÇALIŞMAZ.
    let outcome = AgentExecutor::execute(
        context.clone(),
        "iki şey yap".to_string(),
        test_budget(),
        mk_tools(),
        Some(router.clone()),
        None,
        Some(Arc::new(RiskEngine::new())),
        Some(store.clone()),
        None,
    )
    .await
    .unwrap();
    let approval_id = match outcome {
        AgentOutcome::PendingApproval { approval_id } => approval_id,
        o => panic!("PendingApproval bekleniyordu: {o:?}"),
    };
    assert_eq!(risky_calls.load(Ordering::SeqCst), 0);
    let pending = store.take(&approval_id).unwrap();
    assert!(pending.autonomous, "otonom duraklama işaretlenmeli");

    // 2) Onay + resume: onaylanan adım çalışır, SONRA planlayıcı yeniden
    //    sorulur ve ikinci adım da çalışır, sonunda tamamlanır.
    let outcome = AgentRuntime::resume(
        pending,
        mk_tools(),
        Some(router),
        None,
        Some(Arc::new(RiskEngine::new())),
        Some(store),
        None,
    )
    .await
    .expect("resume hatasız ilerlemeli");

    assert_eq!(outcome, AgentOutcome::Completed);
    assert_eq!(
        risky_calls.load(Ordering::SeqCst),
        1,
        "onaylanan adım bir kez çalışmalı"
    );
    assert_eq!(
        second_calls.load(Ordering::SeqCst),
        1,
        "onaydan sonra döngü devam etmeli"
    );

    // Planlayıcı, onaylanan adımın sonucunu geçmişte görmüş olmalı.
    let seen = prompts.lock().unwrap();
    assert!(
        seen.iter().skip(1).any(|p| p.contains("risky tamam")),
        "onay sonrası istem, onaylanan adımın çıktısını içermeli: {seen:?}"
    );
}

#[tokio::test]
async fn failing_every_step_is_reported_as_failure_not_completed() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools: Vec<Arc<dyn AgentTool>> = vec![Arc::new(CountTool {
        name: "second",
        risk: RiskLevel::Low,
        calls: calls.clone(),
        fail: true,
        cap: None,
    })];
    let (router, _p) = scripted_router(&[STEP_SECOND_RETRYABLE, r#"{"done": true}"#]);
    let audit = Arc::new(AuditLog::new(100));
    let context = ctx();
    let exec_id = context.execution_id;

    let result = AgentExecutor::execute(
        context,
        "başarısız olacak".to_string(),
        test_budget(),
        tools,
        Some(router),
        None,
        Some(Arc::new(RiskEngine::new())),
        None,
        Some(audit.clone()),
    )
    .await;

    assert!(
        result.is_err(),
        "hiçbir adım başarılı olmadıysa başarısız sayılmalı: {result:?}"
    );
    assert!(
        audit
            .list_for_execution(exec_id)
            .iter()
            .any(|e| matches!(e.kind, AuditEventKind::ExecutionFailed { .. }))
    );
}

// ── Dayanıklılık: bozuk yanıt, yeniden deneme, yetki filtresi ──

#[test]
fn extract_json_object_handles_fences_and_prose() {
    assert_eq!(
        extract_json_object(r#"{"done": true}"#),
        r#"{"done": true}"#
    );
    assert_eq!(
        extract_json_object("```json\n{\"done\": true}\n```"),
        r#"{"done": true}"#
    );
    assert_eq!(
        extract_json_object(
            "Tabii! İşte karar: {\"done\": false, \"name\": \"x\"} umarım yardımcı olur"
        ),
        r#"{"done": false, "name": "x"}"#
    );
    assert_eq!(extract_json_object("JSON yok"), "JSON yok");
}

fn tool_with_cap(name: &'static str, cap: Option<AgentCapability>) -> Arc<dyn AgentTool> {
    Arc::new(CountTool {
        name,
        risk: RiskLevel::Low,
        calls: Arc::new(AtomicUsize::new(0)),
        fail: false,
        cap,
    })
}

#[tokio::test]
async fn plan_next_tolerates_prose_missing_done_and_non_string_arguments() {
    let (router, _p) = scripted_router(&[
        "Tabii! ```json\n{\"name\": \"ikinci\", \"tool_name\": \"second\", \"arguments\": [\"b\", 5, true]}\n``` bitti",
    ]);
    let tools = vec![tool_with_cap("second", None)];
    match AgentPlanner::plan_next_detailed("hedef", &[], &router, &tools).await {
        Ok(NextStepDecision::Step(step)) => {
            let call = step.tool_call.expect("tool_call olmalı");
            assert_eq!(call.tool_name, "second");
            assert_eq!(call.arguments, vec!["b", "5", "true"]);
        }
        other => panic!("adım bekleniyordu: {other:?}"),
    }
}

#[tokio::test]
async fn plan_next_detailed_explains_why_it_failed() {
    let (router, _p) = scripted_router(&["tamamen düz metin, JSON yok"]);
    let err = AgentPlanner::plan_next_detailed("hedef", &[], &router, &[])
        .await
        .unwrap_err();
    assert!(err.contains("geçerli JSON değil"), "{err}");
    let empty = Arc::new(ProviderRouter::new());
    let err = AgentPlanner::plan_next_detailed("hedef", &[], &empty, &[])
        .await
        .unwrap_err();
    assert!(err.contains("AI isteği başarısız"), "{err}");
}

#[tokio::test]
async fn planner_retries_a_garbled_reply_instead_of_failing_the_agent() {
    let second_calls = Arc::new(AtomicUsize::new(0));
    let tools: Vec<Arc<dyn AgentTool>> = vec![Arc::new(CountTool {
        name: "second",
        risk: RiskLevel::Low,
        calls: second_calls.clone(),
        fail: false,
        cap: None,
    })];
    // 1. yanıt bozuk, yeniden denemede doğru adım, sonra bitti.
    let (router, prompts) = scripted_router(&["bozuk yanıt", STEP_SECOND, r#"{"done": true}"#]);
    let outcome = AgentExecutor::execute(
        ctx(),
        "hedef".to_string(),
        test_budget(),
        tools,
        Some(router),
        None,
        None,
        None,
        None,
    )
    .await
    .expect("geçici bozuk yanıt agent'ı düşürmemeli");
    assert_eq!(outcome, AgentOutcome::Completed);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
    assert_eq!(prompts.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn same_failing_call_twice_stops_the_agent() {
    let calls = Arc::new(AtomicUsize::new(0));
    let tools: Vec<Arc<dyn AgentTool>> = vec![Arc::new(CountTool {
        name: "second",
        risk: RiskLevel::Low,
        calls: calls.clone(),
        fail: true,
        cap: None,
    })];
    let (router, prompts) = scripted_router(&[
        STEP_SECOND_RETRYABLE,
        STEP_SECOND_RETRYABLE,
        STEP_SECOND_RETRYABLE,
        r#"{"done": true}"#,
    ]);
    let result = AgentExecutor::execute(
        ctx(),
        "hedef".to_string(),
        test_budget(),
        tools,
        Some(router),
        None,
        None,
        None,
        None,
    )
    .await;
    match result {
        Err(RuntimeError::TaskExecutionFailed { message }) => {
            assert!(message.contains("iki kez başarısız"), "{message}");
        }
        other => panic!("başarısızlık bekleniyordu: {other:?}"),
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2, "üçüncü deneme yapılmamalı");
    assert_eq!(prompts.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn planner_only_sees_tools_the_agent_may_use() {
    let agent = ctx();
    let engine = Arc::new(CapabilityEngine::new());
    engine.grant_capability(agent.agent_id, AgentCapability::WorkspaceRead);
    let tools = vec![
        tool_with_cap("okuyucu", Some(AgentCapability::WorkspaceRead)),
        tool_with_cap("yazici", Some(AgentCapability::WorkspaceWrite)),
        tool_with_cap("serbest", None),
    ];
    let (router, prompts) = scripted_router(&[r#"{"done": true}"#]);
    let outcome = AgentExecutor::execute(
        agent,
        "hedef".to_string(),
        test_budget(),
        tools,
        Some(router),
        Some(engine),
        Some(Arc::new(RiskEngine::new())),
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(outcome, AgentOutcome::Completed);
    let seen = prompts.lock().unwrap();
    assert!(
        seen[0].contains("okuyucu") && seen[0].contains("serbest"),
        "{}",
        seen[0]
    );
    assert!(
        !seen[0].contains("yazici"),
        "yetkisiz araç gösterilmemeli: {}",
        seen[0]
    );
}

#[tokio::test]
async fn agent_with_no_usable_tools_fails_with_a_helpful_message() {
    let agent = ctx();
    let engine = Arc::new(CapabilityEngine::new()); // hiç yetki yok
    let tools = vec![
        tool_with_cap("terminalci", Some(AgentCapability::TerminalExecution)),
        tool_with_cap("okuyucu", Some(AgentCapability::WorkspaceRead)),
    ];
    let (router, prompts) = scripted_router(&[r#"{"done": true}"#]);
    let result = AgentExecutor::execute(
        agent,
        "hedef".to_string(),
        test_budget(),
        tools,
        Some(router),
        Some(engine),
        Some(Arc::new(RiskEngine::new())),
        None,
        None,
    )
    .await;
    match result {
        Err(RuntimeError::TaskExecutionFailed { message }) => {
            assert!(message.contains("hiçbir araç yetkisi"), "{message}");
        }
        other => panic!("başarısızlık bekleniyordu: {other:?}"),
    }
    assert!(prompts.lock().unwrap().is_empty(), "AI'ya hiç sorulmamalı");
}

#[test]
fn capability_denial_tells_the_user_which_chip_to_pick() {
    let engine = CapabilityEngine::new();
    let agent = Uuid::new_v4();
    engine.grant_capability(agent, AgentCapability::WorkspaceWrite);
    match engine.check(&agent, Some(AgentCapability::WorkspaceRead)) {
        crate::security::capability_engine::CapabilityDecision::Denied { reason } => {
            assert!(reason.contains("Dosya okuma"), "{reason}");
        }
        other => panic!("{other:?}"),
    }
}
