// ============================================================
// src/wasm/wasmi_engine.rs
//
// Android / Play Store uyumlu WASM backend.
//
// NEDEN wasmi?
//   wasmtime, Cranelift JIT kullanır.
//   Android W^X politikası JIT'i bloke eder.
//   wasmi interpreter tabanlıdır → Android'de çalışır.
//
// wasmi 0.31.x NOT: set_fuel() v0.32'de eklendi.
//   v0.31'de fuel metering yok; timeout ile kaynak sınırlama yapılır.
// ============================================================

use std::sync::Arc;

use async_trait::async_trait;
use tokio::time::timeout;
use tracing::{debug, info};

use wasmi::{Engine, Linker, Module, Store};

use crate::errors::wasm::WasmError;
use crate::task::task::TaskDefinition;
use crate::wasm::module_store::ModuleStore;
use crate::wasm::WasmExecutor;

// ── Host context ──────────────────────────────────────────

#[derive(Clone)]
pub struct WasmiHostContext {
    fuel_limit: u64,
}

// ── Sandbox limitleri ─────────────────────────────────────

pub struct WasmiSandboxLimits {
    pub fuel_limit:        u64,
    pub execution_timeout: std::time::Duration,
}

impl Default for WasmiSandboxLimits {
    fn default() -> Self {
        Self {
            fuel_limit:        10_000_000,
            execution_timeout: std::time::Duration::from_secs(30),
        }
    }
}

// ── Engine ────────────────────────────────────────────────

pub struct WasmiEngine {
    limits:       WasmiSandboxLimits,
    module_store: Arc<ModuleStore>,
    engine:       Engine,
}

impl WasmiEngine {
    pub fn new(
        limits:       WasmiSandboxLimits,
        module_store: Arc<ModuleStore>,
    ) -> Self {
        let engine = Engine::default();
        info!("WasmiEngine başlatıldı (Android/embedded backend)");
        Self { limits, module_store, engine }
    }

    pub fn module_store(&self) -> Arc<ModuleStore> {
        Arc::clone(&self.module_store)
    }

    fn execute_sync(
        &self,
        binary: Arc<Vec<u8>>,
        entrypoint: String,
        _fuel_limit: u64, // wasmi 0.31: fuel metering yok, timeout ile sınırlanır
    ) -> Result<Vec<u8>, WasmError> {
        let module = Module::new(&self.engine, &binary[..])
            .map_err(|e| WasmError::InvalidModule {
                reason: e.to_string(),
            })?;

        let host_ctx = WasmiHostContext { fuel_limit: _fuel_limit };
        let mut store = Store::new(&self.engine, host_ctx);

        let mut linker = Linker::<WasmiHostContext>::new(&self.engine);

        register_wasmi_host_functions(&mut linker)
            .map_err(|e| WasmError::EngineFailure {
                message: e.to_string(),
            })?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|_| WasmError::ExecutionPanic)?
            .start(&mut store)
            .map_err(|_| WasmError::ExecutionPanic)?;

        let func = instance
            .get_typed_func::<(), ()>(&store, &entrypoint)
            .map_err(|_| WasmError::MissingEntrypoint)?;

        func.call(&mut store, ())
            .map_err(|_| WasmError::ExecutionPanic)?;

        debug!(entrypoint = %entrypoint, "wasmi execution OK");
        Ok(Vec::new())
    }
}

// ── WasmExecutor impl ─────────────────────────────────────

#[async_trait]
impl WasmExecutor for WasmiEngine {
    async fn execute(
        &self,
        task: TaskDefinition,
    ) -> Result<Vec<u8>, WasmError> {
        if task.timeout_ms == 0 {
            return Err(WasmError::InvalidConfiguration {
                reason: "timeout_ms must be > 0".into(),
            });
        }

        if !task.has_module() {
            return Err(WasmError::InvalidConfiguration {
                reason: "empty wasm_module_hash — no module registered".into(),
            });
        }

        // ModuleStore'dan binary al (Result döner, ok_or değil map_err)
        let binary = self
            .module_store
            .get(&task.wasm_module_hash)
            .map_err(|_| WasmError::InvalidModule {
                reason: "module not found in store".into(),
            })?;

        let entrypoint   = task.entrypoint.clone();
        let fuel_limit   = self.limits.fuel_limit;
        let exec_timeout = std::time::Duration::from_millis(task.timeout_ms);

        let engine_clone = Engine::default();
        let binary_clone = Arc::clone(&binary);

        let result = timeout(exec_timeout, tokio::task::spawn_blocking(move || {
            let module = Module::new(&engine_clone, &binary_clone[..])
                .map_err(|e| WasmError::InvalidModule {
                    reason: e.to_string(),
                })?;

            let host_ctx = WasmiHostContext { fuel_limit };
            let mut store = Store::new(&engine_clone, host_ctx);

            let mut linker = Linker::<WasmiHostContext>::new(&engine_clone);
            register_wasmi_host_functions(&mut linker)
                .map_err(|e| WasmError::EngineFailure { message: e.to_string() })?;

            let instance = linker
                .instantiate(&mut store, &module)
                .map_err(|_| WasmError::ExecutionPanic)?
                .start(&mut store)
                .map_err(|_| WasmError::ExecutionPanic)?;

            let func = instance
                .get_typed_func::<(), ()>(&store, &entrypoint)
                .map_err(|_| WasmError::MissingEntrypoint)?;

            func.call(&mut store, ())
                .map_err(|_| WasmError::ExecutionPanic)?;

            Ok::<Vec<u8>, WasmError>(Vec::new())
        }))
        .await
        .map_err(|_| WasmError::Timeout)?;

        result.map_err(|_| WasmError::ExecutionPanic)?
    }
}

// ── Host fonksiyonları ────────────────────────────────────

fn register_wasmi_host_functions(
    linker: &mut Linker<WasmiHostContext>,
) -> Result<(), wasmi::Error> {
    linker.func_wrap(
        "aether",
        "log",
        |_caller: wasmi::Caller<'_, WasmiHostContext>,
         level: i32,
         value: i32| {
            let _ = (level, value);
        },
    )?;
    Ok(())
}
