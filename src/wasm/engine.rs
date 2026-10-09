// ============================================================
// src/wasm/engine.rs  (v4)
//
// Değişiklik: WasmExecutor trait implement edildi.
//
// ÖNCE:
//   impl WasmEngine { pub async fn execute(...) }
//   → WorkerExecutor doğrudan Arc<WasmEngine> tutuyordu.
//
// SONRA:
//   impl WasmExecutor for WasmEngine
//   → WorkerExecutor Arc<dyn WasmExecutor> tutar.
//   → Hangi backend derlendiği worker/runtime'a görünmez.
//
// Bu dosya yalnızca backend-wasmtime feature aktifken derlenir.
// ============================================================

use std::sync::Arc;

use dashmap::DashMap;

use tokio::time::timeout;
use tracing::{debug, info};
use wasmtime::{Engine, Instance, Linker, Module, Store, StoreLimitsBuilder};

use async_trait::async_trait;

use crate::errors::wasm::WasmError;
use crate::task::task::TaskDefinition;
use crate::wasm::WasmExecutor;
use crate::wasm::host::{HostContext, register_host_functions};
use crate::wasm::module_store::{ModuleHash, ModuleStore};
use crate::wasm::sandbox::{SandboxLimits, create_engine};

pub struct WasmEngine {
    engine: Arc<Engine>,
    limits: SandboxLimits,
    module_store: Arc<ModuleStore>,
    /// Compiled module cache: hash → Arc<Module>
    compile_cache: DashMap<ModuleHash, Arc<Module>>,
}

impl WasmEngine {
    pub fn new(limits: SandboxLimits, module_store: Arc<ModuleStore>) -> Result<Self, WasmError> {
        let engine = create_engine().map_err(|e| WasmError::EngineFailure {
            message: e.to_string(),
        })?;

        Ok(Self {
            engine: Arc::new(engine),
            limits,
            module_store,
            compile_cache: DashMap::new(),
        })
    }

    pub async fn execute(&self, task: TaskDefinition) -> Result<Vec<u8>, WasmError> {
        // Timeout kontrolü
        if task.timeout_ms == 0 {
            return Err(WasmError::InvalidConfiguration {
                reason: "timeout_ms must be > 0".into(),
            });
        }

        // Boş modül kontrolü — retry storm engeller.
        // Duplicate check kaldırıldı; tek kontrol yeterli.
        if !task.has_module() {
            return Err(WasmError::InvalidConfiguration {
                reason: "empty wasm_module_hash — no module registered".into(),
            });
        }

        // L1: Binary store'dan al
        let binary = self.module_store.get(&task.wasm_module_hash).map_err(|_| {
            WasmError::InvalidConfiguration {
                // InvalidConfiguration → Permanent → retry yok
                reason: "module not found in store".into(),
            }
        })?;

        // L2: Compile cache'den al veya derle
        let module = self.get_or_compile(task.wasm_module_hash, &binary).await?;

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

        let module = tokio::task::spawn_blocking(move || {
            Module::new(&engine, &binary).map_err(|e| WasmError::InvalidModule {
                reason: e.to_string(),
            })
        })
        .await
        .map_err(|_| WasmError::EngineFailure {
            message: "compile task panicked".into(),
        })??;

        let module = Arc::new(module);

        self.compile_cache.insert(hash, Arc::clone(&module));

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

        register_host_functions(&mut linker).map_err(|e| WasmError::EngineFailure {
            message: e.to_string(),
        })?;

        let store_limits = StoreLimitsBuilder::new()
            .memory_size(self.limits.memory_limit_bytes)
            .instances(1)
            .memories(4)
            .tables(16)
            .build();

        let mut store = Store::new(
            &self.engine,
            HostContext {
                limits: store_limits,
            },
        );
        store.limiter(|state| &mut state.limits);

        store
            .set_fuel(self.limits.fuel_limit)
            .map_err(|_| WasmError::ResourceLimitExceeded)?;

        // `create_engine()` epoch_interruption(true) açar; ama hiçbir yer
        // `engine.increment_epoch()` çağırmıyor. Varsayılan deadline ile
        // Wasm ilk epoch kontrolünde hemen trap'e düşebilir ("execution panic").
        // Süre sınırı zaten fuel + tokio timeout ile uygulanıyor; epoch'u fiilen
        // devre dışı bırakmak için deadline'ı sonsuza çekiyoruz.
        store.set_epoch_deadline(u64::MAX);

        let exec_timeout = self.limits.execution_timeout;

        let execution = async {
            let instance: Instance = linker
                .instantiate_async(&mut store, &module)
                .await
                .map_err(|e| {
                    tracing::warn!(err = %e, "wasmtime: instantiate başarısız");
                    WasmError::ExecutionPanic
                })?;

            let function = instance
                .get_typed_func::<(), ()>(&mut store, &task.entrypoint)
                .map_err(|_| WasmError::MissingEntrypoint)?;

            function.call_async(&mut store, ()).await.map_err(|e| {
                tracing::warn!(err = %e, "wasmtime: çağrı trap'e düştü");
                WasmError::ExecutionPanic
            })?;

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

// ── WasmExecutor trait impl ───────────────────────────────
//
// Worker ve runtime katmanları artık sadece bu trait'i görür.
// WasmEngine::execute() → trait metodu — zero-cost forwarding.

#[async_trait]
impl WasmExecutor for WasmEngine {
    async fn execute(&self, task: TaskDefinition) -> Result<Vec<u8>, WasmError> {
        self.execute(task).await
    }
}
