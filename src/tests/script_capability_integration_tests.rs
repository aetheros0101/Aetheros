// ============================================================
// src/tests/script_capability_integration_tests.rs
//
// V10 Sprint 1b: capability_engine_tests.rs'teki test bir AtomicBool
// test-double kullanıyordu (bilinçli olarak — bkz. o dosyanın başı).
// Bu dosya farkı kapatıyor: GERÇEK ScriptTool + GERÇEK WasmExecutor
// backend'i (wasmtime veya wasmi, hangisi derlenmişse) + GERÇEK bir
// WAT modülü, CapabilityEngine'in arkasında fiilen çalışıyor mu diye
// sınar.
//
// bridge/state.rs ile AYNI feature-gate deseni: hangi backend aktifse
// (backend-wasmtime = server/desktop, backend-wasmi = Android/Termux)
// bu test o backend ile derlenir — hiçbiri hardcode edilmedi.
// ============================================================

use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::capabilities::AgentCapability;
use crate::agents::runtime::AgentRuntime;
use crate::scripting::definition::ScriptDefinition;
use crate::scripting::engine::ScriptEngine;
use crate::scripting::tool::ScriptTool;
use crate::security::capability_engine::CapabilityEngine;
use crate::types::agent_tool::AgentTool;
use crate::wasm::module_store::ModuleStore;

#[cfg(feature = "backend-wasmtime")]
use crate::wasm::engine::WasmEngine;
#[cfg(feature = "backend-wasmtime")]
use crate::wasm::sandbox::SandboxLimits;

#[cfg(feature = "backend-wasmi")]
use crate::wasm::wasmi_engine::{WasmiEngine, WasmiSandboxLimits};

fn test_budget() -> AgentExecutionBudget {
    AgentExecutionBudget {
        max_tokens: 10_000,
        max_steps: 10,
        max_runtime_seconds: 30,
    }
}

/// Hiçbir stub yok: gerçek ModuleStore + gerçek WasmExecutor backend'i
/// (derleme zamanında seçilir) + gerçek derlenmiş bir WAT modülünden
/// ScriptTool kurar.
fn make_real_script_tool() -> Arc<dyn AgentTool> {
    let module_store = Arc::new(ModuleStore::new());

    // ScriptEngine::to_task() hash'i script.wasm_binary'den hesaplayıp
    // WasmExecutor'a "bu hash'i module_store'da bul, çalıştır" der —
    // yani binary'nin GERÇEKTEN aynı module_store'da kayıtlı olması
    // şart, yoksa "module not found in store" hatası alınır (bu tam
    // olarak bu düzeltmeden önce bu testin kendisinde patladığı yer).
    // Üretimde bu sorun yaşanmıyor çünkü register_agent_script,
    // compile_wat_to_wasm'ın ZATEN o store'a yazdığı bytes'ı kullanıyor.
    let script = ScriptDefinition::from_wat(
        "noop_script",
        r#"(module (func (export "main")))"#,
        "main",
        1_000,
    )
    .expect("minimal WAT geçerli olmalı");

    module_store
        .store(script.wasm_binary.clone())
        .expect("derlenmiş binary module_store'a yazılabilmeli");

    #[cfg(feature = "backend-wasmtime")]
    let wasm_executor: Arc<dyn crate::wasm::WasmExecutor> = Arc::new(
        WasmEngine::new(
            SandboxLimits {
                memory_limit_bytes: 16 * 1024 * 1024,
                execution_timeout: Duration::from_secs(5),
                fuel_limit: 1_000_000,
            },
            module_store,
        )
        .expect("gerçek WasmEngine kurulabilmeli"),
    );

    #[cfg(feature = "backend-wasmi")]
    let wasm_executor: Arc<dyn crate::wasm::WasmExecutor> = Arc::new(WasmiEngine::new(
        WasmiSandboxLimits {
            fuel_limit: 1_000_000,
            execution_timeout: Duration::from_secs(5),
        },
        module_store,
    ));

    let script_engine = Arc::new(ScriptEngine::new(wasm_executor));

    Arc::new(ScriptTool::new(script, script_engine))
}

#[tokio::test]
async fn real_script_tool_is_denied_without_grant() {
    let tool = make_real_script_tool();
    let agent_id = Uuid::new_v4();
    let engine = Arc::new(CapabilityEngine::new()); // hiç grant yok

    let runtime = AgentRuntime::new(
        test_budget(),
        vec![tool],
        None,
        Some(engine),
        None,
        None,
        None,
    );
    let result = runtime.invoke_best_tool(agent_id, "noop_script").await;

    assert!(
        result.is_err(),
        "grant edilmemiş agent, GERÇEK ScriptTool'u bile çağıramamalı"
    );
}

#[tokio::test]
async fn real_script_tool_actually_runs_on_wasmtime_when_granted() {
    let tool = make_real_script_tool();
    let agent_id = Uuid::new_v4();
    let engine = Arc::new(CapabilityEngine::new());
    engine.grant_capability(agent_id, AgentCapability::WasmExecution);

    let runtime = AgentRuntime::new(
        test_budget(),
        vec![tool],
        None,
        Some(engine),
        None,
        None,
        None,
    );
    let result = runtime.invoke_best_tool(agent_id, "noop_script").await;

    assert!(
        result.is_ok(),
        "grant edilince gerçek WASM modülü wasmtime üzerinde fiilen çalışmalı: {:?}",
        result.err()
    );
}
