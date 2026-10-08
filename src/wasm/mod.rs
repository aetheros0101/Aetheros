// ── Backend modülleri (özellik flag'ine göre derlenir) ────
//
//  backend-wasmtime  →  JIT, Cranelift, server-grade
//  backend-wasmi     →  Interpreter, Android/Play Store uyumlu
//
//  Her iki backend de WasmExecutor trait'ini implement eder;
//  runtime ve worker'lar concrete tip yerine sadece bu
//  trait'e bağlıdır → derleme zamanında seçim yapılır,
//  çalışma zamanında sıfır overhead.
// ─────────────────────────────────────────────────────────

pub mod capabilities;
pub mod module_store;

// ── wasmtime backend (varsayılan: server / desktop) ───────
#[cfg(feature = "backend-wasmtime")]
pub mod engine;
#[cfg(feature = "backend-wasmtime")]
pub mod host;
#[cfg(feature = "backend-wasmtime")]
pub mod sandbox;

// ── wasmi backend (Android / embedded / Play Store) ───────
#[cfg(feature = "backend-wasmi")]
pub mod wasmi_engine;
#[cfg(feature = "backend-wasmi")]
pub mod wasmi_host;

use async_trait::async_trait;

use crate::errors::wasm::WasmError;
use crate::task::task::TaskDefinition;

/// Ortak WASM yürütücü kontratı.
///
/// Her iki backend (wasmtime / wasmi) bu trait'i implement eder.
/// Worker, scripting ve runtime katmanları `Arc<dyn WasmExecutor>`
/// tutar; hangi backend'in derlendiğini bilmek zorunda değildir.
#[async_trait]
pub trait WasmExecutor: Send + Sync {
    async fn execute(&self, task: TaskDefinition) -> Result<Vec<u8>, WasmError>;
}
