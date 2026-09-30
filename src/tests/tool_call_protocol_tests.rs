// ============================================================
// src/tests/tool_call_protocol_tests.rs
//
// V10 Sprint 2: Action/Tool Protocol.
//
// Neden ai_plan()'ın hallucination-filtreleme mantığı burada test
// edilmiyor? O davranış gerçek bir AI provider'ın ürettiği JSON'a
// bağlı — sahte bir ProviderRouter kurmak bu testi ya kırılgan ya da
// anlamsız kılardı. Onun yerine sınanan şey: (1) invoke_tool_call'ın
// GERÇEK bir ToolCall'ı doğru tool'a yönlendirdiği, bilinmeyen bir
// tool'da hata döndürdüğü ve CapabilityEngine'e hâlâ tabi olduğu;
// (2) fallback_plan'ın (router yokken) hiçbir adıma tool_call
// eklemediği — yani tool'suz senaryoda sistem sessizce eskisi gibi
// davranıyor.
// ============================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use uuid::Uuid;

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::capabilities::AgentCapability;
use crate::agents::planner::AgentPlanner;
use crate::agents::plans::ToolCall;
use crate::agents::runtime::AgentRuntime;
use crate::security::capability_engine::CapabilityEngine;
use crate::types::agent_tool::AgentTool;

struct EchoTool {
    invoked_with: Arc<std::sync::Mutex<Option<Vec<String>>>>,
}

#[async_trait]
impl AgentTool for EchoTool {
    fn name(&self) -> &'static str {
        "echo_tool"
    }

    async fn invoke(&self, arguments: Vec<String>) -> Result<String, String> {
        *self.invoked_with.lock().unwrap() = Some(arguments.clone());
        Ok(arguments.join(","))
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
async fn invoke_tool_call_routes_to_matching_tool_with_its_arguments() {
    let invoked_with = Arc::new(std::sync::Mutex::new(None));
    let tool: Arc<dyn AgentTool> = Arc::new(EchoTool {
        invoked_with: invoked_with.clone(),
    });
    let agent_id = Uuid::new_v4();

    let runtime = AgentRuntime::new(test_budget(), vec![tool], None, None, None, None, None);
    let call = ToolCall {
        tool_name: "echo_tool".to_string(),
        arguments: vec!["a".to_string(), "b".to_string()],
    };

    let result = runtime.invoke_tool_call(agent_id, &call).await;

    assert_eq!(result, Ok("a,b".to_string()));
    assert_eq!(
        *invoked_with.lock().unwrap(),
        Some(vec!["a".to_string(), "b".to_string()])
    );
}

#[tokio::test]
async fn invoke_tool_call_errors_on_unknown_tool_name() {
    let tool: Arc<dyn AgentTool> = Arc::new(EchoTool {
        invoked_with: Arc::new(std::sync::Mutex::new(None)),
    });
    let agent_id = Uuid::new_v4();

    let runtime = AgentRuntime::new(test_budget(), vec![tool], None, None, None, None, None);
    let call = ToolCall {
        tool_name: "does_not_exist".to_string(),
        arguments: vec![],
    };

    let result = runtime.invoke_tool_call(agent_id, &call).await;

    assert!(
        result.is_err(),
        "runtime'da olmayan bir tool_name sessizce geçilmemeli, hata dönmeli"
    );
}

#[tokio::test]
async fn invoke_tool_call_still_respects_capability_engine() {
    struct RiskyEcho {
        invoked: Arc<AtomicBool>,
    }

    #[async_trait]
    impl AgentTool for RiskyEcho {
        fn name(&self) -> &'static str {
            "risky_echo"
        }
        fn required_capability(&self) -> Option<AgentCapability> {
            Some(AgentCapability::WasmExecution)
        }
        async fn invoke(&self, _arguments: Vec<String>) -> Result<String, String> {
            self.invoked.store(true, Ordering::SeqCst);
            Ok("ran".to_string())
        }
    }

    let invoked = Arc::new(AtomicBool::new(false));
    let tool: Arc<dyn AgentTool> = Arc::new(RiskyEcho {
        invoked: invoked.clone(),
    });
    let agent_id = Uuid::new_v4();
    let engine = Arc::new(CapabilityEngine::new()); // grant yok

    let runtime = AgentRuntime::new(test_budget(), vec![tool], None, Some(engine), None, None, None);
    let call = ToolCall {
        tool_name: "risky_echo".to_string(),
        arguments: vec![],
    };

    let result = runtime.invoke_tool_call(agent_id, &call).await;

    assert!(
        result.is_err(),
        "invoke_tool_call, invoke_best_tool ile AYNI capability denetiminden geçmeli"
    );
    assert!(!invoked.load(Ordering::SeqCst));
}

#[tokio::test]
async fn fallback_plan_never_attaches_a_tool_call() {
    // router = None → fallback_plan devreye girer.
    let plan = AgentPlanner::plan("herhangi bir hedef".to_string(), None, &[]).await;

    assert!(
        plan.planned_steps.iter().all(|s| s.tool_call.is_none()),
        "tool listesi olmayan / router olmayan fallback plan hiçbir adıma tool_call eklememeli"
    );
}
