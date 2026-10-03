// ============================================================
// src/tests/capability_engine_tests.rs
//
// V10 Sprint 1: CapabilityEngine'in gerçek çağrı yolunda
// (AgentRuntime::invoke_best_tool) enforce edildiğini kanıtlar.
//
// NEDEN gerçek ScriptTool/wasmtime değil, test-double bir tool?
//   invoke_best_tool() capability kontrolünü invoke()'dan ÖNCE
//   yapıyor — yani asıl kanıtlanması gereken şey "reddedilen bir
//   çağrı invoke()'a hiç ulaşmıyor" (WASM'ın kendisi değil).
//   Gerçek ScriptTool + wasmtime kurmak bu testi feature seçimine
//   (backend-wasmtime/backend-wasmi) bağımlı ve yavaş kılardı.
//   Bu test-double, "invoke çağrıldı mı" sorusunu bir AtomicBool
//   ile doğrudan ve deterministik ölçer. ScriptTool'un kendi
//   required_capability() override'ı zaten prod kodunda (bkz.
//   src/scripting/tool.rs) — bunun tam WASM'lı entegrasyon testi
//   ayrı bir sprintte scripting_tests.rs'e eklenebilir.
// ============================================================

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::capabilities::{AgentCapabilities, AgentCapability};
use crate::agents::runtime::AgentRuntime;
use crate::security::capability_engine::CapabilityEngine;
use crate::types::agent_tool::AgentTool;

/// invoke() gerçekten çağrılırsa `invoked` true olur — testin ölçtüğü
/// tek şey bu: capability reddi, invoke()'a ulaşmadan mı duruyor?
struct RiskyToolDouble {
    invoked: Arc<AtomicBool>,
}

#[async_trait]
impl AgentTool for RiskyToolDouble {
    fn name(&self) -> &'static str {
        "risky_tool"
    }

    fn required_capability(&self) -> Option<AgentCapability> {
        Some(AgentCapability::WasmExecution)
    }

    async fn invoke(&self, _arguments: Vec<String>) -> Result<String, String> {
        self.invoked.store(true, Ordering::SeqCst);
        Ok("executed".to_string())
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
async fn denies_tool_when_agent_has_no_grant() {
    let invoked = Arc::new(AtomicBool::new(false));
    let tool: Arc<dyn AgentTool> = Arc::new(RiskyToolDouble {
        invoked: invoked.clone(),
    });
    let agent_id = Uuid::new_v4();

    // Hiç grant yapılmamış CapabilityEngine — deny-by-default.
    let engine = Arc::new(CapabilityEngine::new());
    let runtime = AgentRuntime::new(test_budget(), vec![tool], None, Some(engine), None, None, None);

    let result = runtime.invoke_best_tool(agent_id, "risky_tool").await;

    assert!(
        result.is_err(),
        "grant edilmemiş agent, WasmExecution gerektiren tool'u çağıramamalı"
    );
    assert!(
        !invoked.load(Ordering::SeqCst),
        "reddedilen çağrı, invoke()'a hiç ulaşmamalı"
    );
}

#[tokio::test]
async fn allows_tool_when_agent_has_matching_grant() {
    let invoked = Arc::new(AtomicBool::new(false));
    let tool: Arc<dyn AgentTool> = Arc::new(RiskyToolDouble {
        invoked: invoked.clone(),
    });
    let agent_id = Uuid::new_v4();

    let engine = Arc::new(CapabilityEngine::new());
    engine.grant(
        agent_id,
        AgentCapabilities {
            workflow_execution: false,
            wasm_execution: true,
            ai_reasoning: false,
            remote_execution: false,
            terminal_execution: false,
        },
    );
    let runtime = AgentRuntime::new(test_budget(), vec![tool], None, Some(engine), None, None, None);

    let result = runtime.invoke_best_tool(agent_id, "risky_tool").await;

    assert!(
        result.is_ok(),
        "WasmExecution grant edilmiş agent, tool'u çağırabilmeli"
    );
    assert!(
        invoked.load(Ordering::SeqCst),
        "grant varken invoke() gerçekten çalışmalı"
    );
}

#[tokio::test]
async fn missing_capability_engine_preserves_pre_v10_behavior() {
    let invoked = Arc::new(AtomicBool::new(false));
    let tool: Arc<dyn AgentTool> = Arc::new(RiskyToolDouble {
        invoked: invoked.clone(),
    });
    let agent_id = Uuid::new_v4();

    // capability_engine = None → eski (V9) davranış: kontrol atlanır.
    let runtime = AgentRuntime::new(test_budget(), vec![tool], None, None, None, None, None);

    let result = runtime.invoke_best_tool(agent_id, "risky_tool").await;

    assert!(
        result.is_ok(),
        "capability_engine bağlanmamışsa geriye dönük uyumluluk korunmalı"
    );
    assert!(invoked.load(Ordering::SeqCst));
}
