use std::sync::Arc;

use crate::errors::wasm::WasmError;
use crate::task::task::TaskDefinition;
use crate::wasm::engine::WasmEngine;

pub struct WorkerExecutor {
    engine: Arc<WasmEngine>,
}

impl WorkerExecutor {
    pub fn new(engine: Arc<WasmEngine>) -> Self {
        Self { engine }
    }

    pub async fn execute(
        &self,
        task: TaskDefinition,
    ) -> Result<Vec<u8>, WasmError> {
        self.engine
            .execute(task)
            .await
            .map_err(|error| {
                WasmError::ExecutionFailure {
                    message:
                        error.to_string(),
                }
            })
    }
}
