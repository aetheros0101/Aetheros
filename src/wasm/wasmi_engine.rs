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
use tracing::info;

use wasmi::{Engine, Linker, Module, Store};

use crate::errors::wasm::WasmError;
use crate::task::task::TaskDefinition;
use crate::wasm::WasmExecutor;
use crate::wasm::module_store::ModuleStore;

// ── Host context ──────────────────────────────────────────

#[derive(Clone)]
pub struct WasmiHostContext {
    /// wasmi 0.31'de fuel metering yok — bu alan bilinçli olarak
    /// hazır tutuluyor (ileride epoch-interruption/fuel wiring için),
    /// şu an okunmuyor. Zaman aşımı `execute()` içindeki
    /// `tokio::time::timeout` ile sağlanıyor (bkz. madde #5:
    /// bu, çalışan bir native thread'i zorla durdurmaz — bilinen sınır).
    #[allow(dead_code)]
    fuel_limit: u64,
}

// ── Sandbox limitleri ─────────────────────────────────────

pub struct WasmiSandboxLimits {
    pub fuel_limit: u64,
    pub execution_timeout: std::time::Duration,
}

impl Default for WasmiSandboxLimits {
    fn default() -> Self {
        Self {
            fuel_limit: 10_000_000,
            execution_timeout: std::time::Duration::from_secs(30),
        }
    }
}

// ── Engine ────────────────────────────────────────────────

pub struct WasmiEngine {
    limits: WasmiSandboxLimits,
    module_store: Arc<ModuleStore>,
    /// `execute()` her çağrıda kendi `Engine::default()` kopyasını
    /// oluşturuyor (spawn_blocking sınırı yüzünden), bu yüzden bu alan
    /// şu an okunmuyor. Gelecekte engine'i thread'ler arasında paylaşıp
    /// her task için yeniden oluşturmayı önlemek için burada tutuluyor.
    #[allow(dead_code)]
    engine: Engine,
}

impl WasmiEngine {
    pub fn new(limits: WasmiSandboxLimits, module_store: Arc<ModuleStore>) -> Self {
        let engine = Engine::default();
        info!("WasmiEngine başlatıldı (Android/embedded backend)");
        Self {
            limits,
            module_store,
            engine,
        }
    }

    pub fn module_store(&self) -> Arc<ModuleStore> {
        Arc::clone(&self.module_store)
    }

    // NOT: Burada daha önce `execute_sync` adında, aşağıdaki
    // `WasmExecutor::execute()` ile neredeyse birebir aynı mantığı
    // tekrar eden, hiçbir yerden çağrılmayan bir metod vardı (derleme
    // uyarısı: "method execute_sync is never used"). Gerçek yürütme
    // yolu `execute()` — `spawn_blocking` içinde kendi `Engine::default()`
    // kopyasını oluşturuyor (self.engine'i DEĞİL). Kafa karıştırıcı ölü
    // kod olduğu için kaldırıldı; `self.engine` alanı şu an sadece bu
    // yapının kurucusunda set ediliyor, aktif olarak okunmuyor
    // (bkz. alandaki #[allow(dead_code)] notu).
}

// ── WasmExecutor impl ─────────────────────────────────────

#[async_trait]
impl WasmExecutor for WasmiEngine {
    async fn execute(&self, task: TaskDefinition) -> Result<Vec<u8>, WasmError> {
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
        let binary = self.module_store.get(&task.wasm_module_hash).map_err(|_| {
            WasmError::InvalidModule {
                reason: "module not found in store".into(),
            }
        })?;

        let entrypoint = task.entrypoint.clone();
        let fuel_limit = self.limits.fuel_limit;
        let exec_timeout = std::time::Duration::from_millis(task.timeout_ms);

        let engine_clone = Engine::default();
        let binary_clone = Arc::clone(&binary);

        let result = timeout(
            exec_timeout,
            tokio::task::spawn_blocking(move || {
                let module = Module::new(&engine_clone, &binary_clone[..]).map_err(|e| {
                    WasmError::InvalidModule {
                        reason: e.to_string(),
                    }
                })?;

                let host_ctx = WasmiHostContext { fuel_limit };
                let mut store = Store::new(&engine_clone, host_ctx);

                let mut linker = Linker::<WasmiHostContext>::new(&engine_clone);
                register_wasmi_host_functions(&mut linker).map_err(|e| {
                    WasmError::EngineFailure {
                        message: e.to_string(),
                    }
                })?;

                let instance = linker
                    .instantiate(&mut store, &module)
                    .map_err(|_| WasmError::ExecutionPanic)?
                    .start(&mut store)
                    .map_err(|_| WasmError::ExecutionPanic)?;

                let output = call_entrypoint(&mut store, &instance, &entrypoint)?;

                Ok::<Vec<u8>, WasmError>(output)
            }),
        )
        .await
        .map_err(|_| WasmError::Timeout)?;

        result.map_err(|_| WasmError::ExecutionPanic)?
    }
}

// ── Entrypoint çağırma — çoklu imza desteği ───────────────
//
// WASM modülleri farklı dönüş tipleriyle export edilebilir:
//   (func (export "run") (result i32) ...)   ← en yaygın
//   (func (export "run") (result i64) ...)
//   (func (export "run") (result f32) ...)
//   (func (export "run") (result f64) ...)
//   (func (export "run") ...)                ← dönüş yok
//
// get_typed_func<(), ()> SADECE dönüşsüz fonksiyonları kabul
// eder; i32 dönen "Merhaba Dünya" gibi temel örnekler bile
// MissingEntrypoint ile başarısız olurdu. Bu fonksiyon en
// yaygın imzaları sırayla dener ve sonucu little-endian byte
// dizisine kodlar (TaskStatusResponse içinde gösterilebilir).
fn call_entrypoint(
    store: &mut Store<WasmiHostContext>,
    instance: &wasmi::Instance,
    entrypoint: &str,
) -> Result<Vec<u8>, WasmError> {
    // 1) Parametresiz, dönüşsüz: () -> ()
    if let Ok(f) = instance.get_typed_func::<(), ()>(&*store, entrypoint) {
        f.call(&mut *store, ())
            .map_err(|_| WasmError::ExecutionPanic)?;
        return Ok(Vec::new());
    }

    // 2) () -> i32  (en yaygın — "Merhaba Dünya", Fibonacci, vb.)
    if let Ok(f) = instance.get_typed_func::<(), i32>(&*store, entrypoint) {
        let r = f
            .call(&mut *store, ())
            .map_err(|_| WasmError::ExecutionPanic)?;
        return Ok(r.to_le_bytes().to_vec());
    }

    // 3) () -> i64
    if let Ok(f) = instance.get_typed_func::<(), i64>(&*store, entrypoint) {
        let r = f
            .call(&mut *store, ())
            .map_err(|_| WasmError::ExecutionPanic)?;
        return Ok(r.to_le_bytes().to_vec());
    }

    // NOT: f32/f64 wasmi 0.31'de WasmResults trait'ini implement
    // etmiyor (sadece (), i32, i64, u32, u64 desteklenir).
    // (result f32)/(result f64) export eden modüller v1'de
    // desteklenmez — MissingEntrypoint döner.

    // Hiçbiri eşleşmedi — export yok veya desteklenmeyen imza
    // (örn. f32/f64 dönüşü veya parametre alan fonksiyonlar)
    Err(WasmError::MissingEntrypoint)
}

// ── Host fonksiyonları ────────────────────────────────────

fn register_wasmi_host_functions(
    linker: &mut Linker<WasmiHostContext>,
) -> Result<(), wasmi::Error> {
    linker.func_wrap(
        "aether",
        "log",
        |_caller: wasmi::Caller<'_, WasmiHostContext>, level: i32, value: i32| {
            let _ = (level, value);
        },
    )?;
    Ok(())
}
