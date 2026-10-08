// ============================================================
// src/scripting/engine.rs
//
// ScriptEngine: ScriptDefinition → WasmExecutor → ScriptResult
//
// WasmEngine → Arc<dyn WasmExecutor>
// Hangi backend (wasmtime/wasmi) olduğu bu katmana görünmez.
// ============================================================

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use tracing::{debug, info};
use uuid::Uuid;

use crate::errors::wasm::WasmError;
use crate::scripting::definition::ScriptDefinition;
use crate::scripting::result::ScriptResult;
use crate::task::priority::TaskPriority;
use crate::task::retry::RetryPolicy;
use crate::task::task::{TaskDefinition, TaskMetadata, TaskState};
use crate::types::ids::TaskId;
use crate::wasm::WasmExecutor;

pub struct ScriptEngine {
    wasm: Arc<dyn WasmExecutor>,
}

impl ScriptEngine {
    pub fn new(wasm: Arc<dyn WasmExecutor>) -> Self {
        Self { wasm }
    }

    /// Script'i çalıştır → ScriptResult döndür.
    ///
    /// Execution süresi ölçülür.
    /// WasmEngine hatası → WasmError olarak iletilir.
    pub async fn run(&self, script: &ScriptDefinition) -> Result<ScriptResult, WasmError> {
        info!(
            script_name = %script.name,
            entrypoint = %script.entrypoint,
            timeout_ms = script.timeout_ms,
            "Running script"
        );

        let task = self.to_task(script);
        let start = Instant::now();

        let output = self.wasm.execute(task).await?;

        let elapsed_ms = start.elapsed().as_millis() as u64;

        debug!(
            script_name = %script.name,
            elapsed_ms,
            output_bytes = output.len(),
            "Script completed"
        );

        Ok(ScriptResult {
            script_name: script.name.clone(),
            output,
            elapsed_ms,
            executed_at: Utc::now(),
        })
    }

    /// ScriptDefinition → TaskDefinition.
    ///
    /// Script'ler Normal öncelikle çalışır.
    /// Retry yok (script'ler idempotent değil sayılır).
    fn to_task(&self, script: &ScriptDefinition) -> TaskDefinition {
        TaskDefinition {
            id: TaskId(Uuid::new_v4()),
            parent: None,
            orchestration: None,
            priority: TaskPriority::Normal,
            deadline: None,
            timeout_ms: script.timeout_ms,
            retry_policy: RetryPolicy {
                max_attempts: 1,
                base_delay_ms: 0,
                max_delay_ms: 0,
                jitter: false,
            },
            metadata: TaskMetadata {
                labels: [("script_name".to_string(), script.name.clone())]
                    .into_iter()
                    .collect(),
            },
            wasm_module_hash: {
                use sha2::{Digest, Sha256};
                let hash: [u8; 32] = Sha256::digest(&script.wasm_binary).into();
                hash
            },
            entrypoint: script.entrypoint.clone(),
            state: TaskState::Queued,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
