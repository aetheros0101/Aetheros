pub mod capabilities;
pub mod engine;
pub mod errors;
pub mod host;
pub mod linker;
pub mod memory;
pub mod sandbox;
pub mod state;
pub mod module_store;

use async_trait::async_trait;

use crate::errors::wasm::WasmError;
use crate::task::task::TaskDefinition;

#[async_trait]
pub trait WasmExecutor {
    async fn execute(
        &self,
        task: TaskDefinition,
    ) -> Result<Vec<u8>, WasmError>;
}
