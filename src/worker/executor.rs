// ── WorkerExecutor — backend-agnostic ────────────────────
//
// Arc<WasmEngine> → Arc<dyn WasmExecutor>
//
// Bu dosya hangi WASM backend'inin derlendiğini bilmez.
// Yalnızca WasmExecutor trait'i üzerinden çalışır.
// Derleme zamanında doğru implementasyon seçilir.

use std::sync::Arc;

use crate::errors::wasm::WasmError;
use crate::task::task::TaskDefinition;
use crate::wasm::WasmExecutor;

pub struct WorkerExecutor {
    engine: Arc<dyn WasmExecutor>,
}

impl WorkerExecutor {
    pub fn new(engine: Arc<dyn WasmExecutor>) -> Self {
        Self { engine }
    }

    pub async fn execute(&self, task: TaskDefinition) -> Result<Vec<u8>, WasmError> {
        // WasmError olduğu gibi döndürülüyor.
        // InvalidModule / InvalidConfiguration → Permanent → retry yok.
        // Retryable → RetryPolicy karar verir.
        self.engine.execute(task).await
    }
}
