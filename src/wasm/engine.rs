// ============================================================
// src/wasm/engine.rs  (v3)
//
// Optimizasyon #1: ModuleStore entegrasyonu
//
// ÖNCE:
//   execute(task) → get_or_compile(&task.wasm_module)
//   Binary task içinde taşınıyordu.
//
// SONRA:
//   execute(task) → module_store.get(task.wasm_module_hash)
//                → get_or_compile(&binary)
//   Binary ModuleStore'dan Arc<Vec<u8>> olarak alınır.
//   get_or_compile zaten SHA-256 compile cache'e sahip.
//   Aynı hash → store lookup O(1) + cache hit O(1).
//
// İki cache katmanı:
//   L1: DashMap<ModuleHash, Arc<Vec<u8>>>  → binary store
//   L2: DashMap<ModuleHash, Arc<Module>>   → compiled cache
//   İkisi de aynı ModuleHash key'i kullanır.
// ============================================================

use std::sync::Arc;

use dashmap::DashMap;

use tokio::time::timeout;
use tracing::{
    debug,
    info,
};
use wasmtime::{
    Engine,
    Instance,
    Linker,
    Module,
    Store,
};

use crate::errors::wasm::WasmError;
use crate::task::task::TaskDefinition;
use crate::wasm::host::{
    register_host_functions,
    HostContext,
};
use crate::wasm::module_store::{
    ModuleHash,
    ModuleStore,
};
use crate::wasm::sandbox::{
    create_engine,
    SandboxLimits,
};

pub struct WasmEngine {
    engine: Arc<Engine>,
    limits: SandboxLimits,
    module_store: Arc<ModuleStore>,
    /// Compiled module cache: hash → Arc<Module>
    compile_cache: DashMap<ModuleHash, Arc<Module>>,
}

impl WasmEngine {
    pub fn new(
        limits: SandboxLimits,
        module_store: Arc<ModuleStore>,
    ) -> Result<Self, WasmError> {
        let engine = create_engine()
            .map_err(|e| WasmError::EngineFailure {
                message: e.to_string(),
            })?;

        Ok(Self {
            engine: Arc::new(engine),
            limits,
            module_store,
            compile_cache: DashMap::new(),
        })
    }

    pub async fn execute(
        &self,
        task: TaskDefinition,
    ) -> Result<Vec<u8>, WasmError> {
        // Timeout kontrolü
        if task.timeout_ms == 0 {
            return Err(WasmError::InvalidConfiguration {
                reason: "timeout_ms must be > 0".into(),
            });
        }

        // Boş modül kontrolü — retry storm engeller
            if !task.has_module() {
                return Err(WasmError::InvalidConfiguration {
                    reason: "empty wasm module hash — no module registered".into(),
                });
            }

        // Modül var mı?
        if !task.has_module() {
            return Err(WasmError::InvalidConfiguration {
                reason: "task has no wasm_module_hash".into(),
            });
        }

        // L1: Binary store'dan al
        let binary = self
            .module_store
            .get(&task.wasm_module_hash)
            .map_err(|_| WasmError::InvalidConfiguration {
                // ÖNCE: WasmError::InvalidModule
                // SONRA: InvalidConfiguration → Permanent → retry yok
                reason: "module not found in store".into(),
            })?;;

        // L2: Compile cache'den al veya derle
        let module = self
            .get_or_compile(task.wasm_module_hash, &binary)
            .await?;

        self.run_module(module, task).await
    }

    async fn get_or_compile(
        &self,
        hash: ModuleHash,
        binary: &[u8],
    ) -> Result<Arc<Module>, WasmError> {
        // Cache hit (lock-free read)
        if let Some(m) = self.compile_cache.get(&hash) {
            debug!(
                hash = %hex::encode(hash),
                "Module compile cache hit"
            );
            return Ok(Arc::clone(&m));
        }

        // Cache miss → Cranelift JIT (blocking)
        let engine = Arc::clone(&self.engine);
        let binary = binary.to_vec();

        let module =
            tokio::task::spawn_blocking(move || {
                Module::new(&engine, &binary)
                    .map_err(|_| WasmError::InvalidModule)
            })
            .await
            .map_err(|_| WasmError::EngineFailure {
                message: "compile task panicked".into(),
            })??;

        let module = Arc::new(module);

        self.compile_cache
            .insert(hash, Arc::clone(&module));

        info!(
            hash = %hex::encode(hash),
            "Module compiled and cached"
        );

        Ok(module)
    }

    async fn run_module(
        &self,
        module: Arc<Module>,
        task: TaskDefinition,
    ) -> Result<Vec<u8>, WasmError> {
        let mut linker = Linker::new(&self.engine);

        register_host_functions(&mut linker)
            .map_err(|e| WasmError::EngineFailure {
                message: e.to_string(),
            })?;

        let mut store =
            Store::new(&self.engine, HostContext);

        store
            .set_fuel(self.limits.fuel_limit)
            .map_err(|_| WasmError::ResourceLimitExceeded)?;

        let exec_timeout = self.limits.execution_timeout;

        let execution = async {
            let instance: Instance = linker
                .instantiate_async(&mut store, &module)
                .await
                .map_err(|_| WasmError::ExecutionPanic)?;

            let function = instance
                .get_typed_func::<(), ()>(
                    &mut store,
                    &task.entrypoint,
                )
                .map_err(|_| WasmError::MissingEntrypoint)?;

            function
                .call_async(&mut store, ())
                .await
                .map_err(|_| WasmError::ExecutionPanic)?;

            Ok::<Vec<u8>, WasmError>(Vec::new())
        };

        timeout(exec_timeout, execution)
            .await
            .map_err(|_| WasmError::Timeout)?
    }

    pub fn cached_module_count(&self) -> usize {
        self.compile_cache.len()
    }

    pub fn module_store(&self) -> Arc<ModuleStore> {
        Arc::clone(&self.module_store)
    }
}
