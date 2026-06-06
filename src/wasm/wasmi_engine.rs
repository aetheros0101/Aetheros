// ============================================================
// src/wasm/wasmi_engine.rs
//
// Android / Play Store uyumlu WASM backend.
//
// NEDEN wasmi?
//   wasmtime, Cranelift JIT kullanır.
//   Android W^X (Write XOR Execute) politikası JIT'i bloke eder.
//   wasmi interpreter tabanlıdır → Android'de çalışır.
//   Play Store: dinamik kod üretimi yasak → wasmi geçer.
//
// FARK:
//   wasmtime  → JIT, ~3-5x daha hızlı, server için ideal
//   wasmi     → Interpreter, ~3-5x daha yavaş AMA mobilde çalışır
//   Mobil iş yükünde (kısa scriptler, otomasyon) bu fark önemsiz.
//
// ASYNC:
//   wasmi'nin kendi async runtime'ı yok.
//   tokio::task::spawn_blocking ile WASM yürütmesi thread pool'a
//   taşınır → tokio event loop bloke olmaz.
//
// FUEL (kaynak sınırlama):
//   wasmtime'daki fuel_limit karşılığı wasmi'de de var.
//   store.set_fuel(n) → N instruction sonra trap.
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

// ── Host context (wasmi sürümü) ───────────────────────────

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
    /// wasmi Engine paylaşımlı — thread-safe, clone ucuz.
    engine:       Engine,
}

impl WasmiEngine {
    pub fn new(
        limits:       WasmiSandboxLimits,
        module_store: Arc<ModuleStore>,
    ) -> Self {
        // wasmi Engine::default() yeterli;
        // JIT config yok (interpreter), cfg sadece fuel/memory.
        let engine = Engine::default();

        info!("WasmiEngine başlatıldı (Android/embedded backend)");

        Self { limits, module_store, engine }
    }

    pub fn module_store(&self) -> Arc<ModuleStore> {
        Arc::clone(&self.module_store)
    }

    // ── İç yürütme mantığı ───────────────────────────────

    fn execute_sync(
        &self,
        binary: Arc<Vec<u8>>,
        entrypoint: String,
        fuel_limit: u64,
    ) -> Result<Vec<u8>, WasmError> {
        // 1. Modülü derle (wasmi'de "derleme" = parse + validate,
        //    JIT yok — bu adım wasmtime'a göre çok daha hızlı)
        let module = Module::new(&self.engine, &binary[..])
            .map_err(|e| WasmError::InvalidModule {
                reason: e.to_string(),
            })?;

        // 2. Store + fuel
        let host_ctx = WasmiHostContext { fuel_limit };
        let mut store = Store::new(&self.engine, host_ctx);

        store
            .set_fuel(fuel_limit)
            .map_err(|_| WasmError::ResourceLimitExceeded)?;

        // 3. Linker + host fonksiyonları
        let mut linker = Linker::<WasmiHostContext>::new(&self.engine);

        register_wasmi_host_functions(&mut linker)
            .map_err(|e| WasmError::EngineFailure {
                message: e.to_string(),
            })?;

        // 4. Instantiate
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| WasmError::ExecutionPanic)? // linker hataları
            .start(&mut store)                         // _start / __wasi_init
            .map_err(|_| WasmError::ExecutionPanic)?;

        // 5. Entrypoint bul
        let func = instance
            .get_typed_func::<(), ()>(&store, &entrypoint)
            .map_err(|_| WasmError::MissingEntrypoint)?;

        // 6. Çalıştır — fuel biterse FuelExhausted trap döner
        func.call(&mut store, ())
            .map_err(|trap| {
                // Fuel tükenmesi → ResourceLimitExceeded
                // Diğer trap → ExecutionPanic
                if trap.to_string().contains("fuel") {
                    WasmError::ResourceLimitExceeded
                } else {
                    WasmError::ExecutionPanic
                }
            })?;

        debug!(
            entrypoint = %entrypoint,
            "wasmi execution OK"
        );

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
        // Temel kontroller (wasmtime backend ile aynı)
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

        // ModuleStore'dan binary al
        let binary = self
            .module_store
            .get(&task.wasm_module_hash)
            .ok_or(WasmError::InvalidModule {
                reason: "module not found in store".into(),
            })?;

        let entrypoint  = task.entrypoint.clone();
        let fuel_limit  = self.limits.fuel_limit;
        let exec_timeout = std::time::Duration::from_millis(task.timeout_ms);

        // wasmi sync → tokio thread pool
        // Event loop'u bloke etmez, Android UI thread güvende.
        let engine_clone = Engine::default(); // Engine::clone ucuz
        let binary_clone = Arc::clone(&binary);

        let result = timeout(exec_timeout, tokio::task::spawn_blocking(move || {
            // Closure: sync wasmi yürütmesi
            let module = Module::new(&engine_clone, &binary_clone[..])
                .map_err(|e| WasmError::InvalidModule { reason: e.to_string() })?;

            let host_ctx = WasmiHostContext { fuel_limit };
            let mut store = Store::new(&engine_clone, host_ctx);
            store.set_fuel(fuel_limit)
                .map_err(|_| WasmError::ResourceLimitExceeded)?;

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
                .map_err(|trap| {
                    if trap.to_string().contains("fuel") {
                        WasmError::ResourceLimitExceeded
                    } else {
                        WasmError::ExecutionPanic
                    }
                })?;

            Ok::<Vec<u8>, WasmError>(Vec::new())
        }))
        .await
        .map_err(|_| WasmError::Timeout)?; // timeout

        // JoinError (thread panic)
        result
            .map_err(|_| WasmError::ExecutionPanic)?
    }
}

// ── Wasmi host fonksiyonları ──────────────────────────────
//
// wasmtime'daki host.rs karşılığı.
// Şimdilik aether::log stub — ileride log seviyesi,
// memory read/write, event emit genişletilebilir.

fn register_wasmi_host_functions(
    linker: &mut Linker<WasmiHostContext>,
) -> Result<(), wasmi::Error> {
    // aether::log(level: i32, value: i32)
    linker.func_wrap(
        "aether",
        "log",
        |_caller: wasmi::Caller<'_, WasmiHostContext>,
         level: i32,
         value: i32| {
            // Gelecekte: caller.data().log_sink.push(...)
            let _ = (level, value);
        },
    )?;

    Ok(())
}
