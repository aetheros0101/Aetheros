// ============================================================
// src/scripting/tool.rs
//
// ScriptTool: AgentTool trait impl.
// Agent, bir script'i araç olarak çağırabilir.
//
// AgentRuntime::invoke_best_tool() →
//   tool.name() == step.name → tool.invoke() →
//   ScriptEngine::run() → output String
// ============================================================

use std::sync::Arc;

use async_trait::async_trait;
use tracing::warn;

use crate::agents::tools::AgentTool;
use crate::scripting::definition::ScriptDefinition;
use crate::scripting::engine::ScriptEngine;

pub struct ScriptTool {
    script: ScriptDefinition,
    engine: Arc<ScriptEngine>,
}

impl ScriptTool {
    pub fn new(
        script: ScriptDefinition,
        engine: Arc<ScriptEngine>,
    ) -> Self {
        Self { script, engine }
    }
}

#[async_trait]
impl AgentTool for ScriptTool {
    fn name(&self) -> &'static str {
        // AgentTool trait'i &'static str bekliyor.
        // String'i sızdırarak static lifetime elde ediyoruz.
        // ScriptTool'lar uzun ömürlü olduğundan kabul edilebilir.
        Box::leak(self.script.name.clone().into_boxed_str())
    }

    /// ScriptTool, keyfi WASM binary çalıştırır — bu yüzden agent'ın
    /// WasmExecution capability'sine sahip olması şart. Grant edilmemişse
    /// CapabilityEngine bu çağrıyı invoke()'a hiç ulaşmadan reddeder.
    fn required_capability(&self) -> Option<crate::agents::capabilities::AgentCapability> {
        Some(crate::agents::capabilities::AgentCapability::WasmExecution)
    }

    async fn invoke(
        &self,
        _args: Vec<String>,
    ) -> Result<String, String> {
        match self.engine.run(&self.script).await {
            Ok(result) => Ok(result.output_as_string()),
            Err(e) => {
                warn!(
                    script = %self.script.name,
                    error = ?e,
                    "ScriptTool invocation failed"
                );
                Err(e.to_string())
            }
        }
    }
}
