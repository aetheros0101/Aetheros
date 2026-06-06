// ============================================================
// src/wasm/wasmi_host.rs
//
// wasmi backend için host fonksiyon tanımları.
//
// wasmtime backend'deki host.rs + linker.rs kombinasyonunun
// wasmi karşılığıdır.
//
// Host fonksiyonlar: WASM modülünün çağırabileceği Rust
// taraflı fonksiyonlardır (print, log, HTTP, DB erişimi vb.)
// ============================================================

use wasmi::{Caller, Linker, Store};

use crate::errors::wasm::WasmError;

/// wasmi host context — store data olarak tutulur.
pub struct WasmiHostContext {
    /// Modülün harcayabileceği maksimum fuel (instruction sayısı)
    pub fuel_limit: u64,
    /// Çıktı tamponu — `aetheros_log` host çağrısı buraya yazar
    pub output: Vec<u8>,
}

impl WasmiHostContext {
    pub fn new(fuel_limit: u64) -> Self {
        Self {
            fuel_limit,
            output: Vec::new(),
        }
    }
}

/// Host import'larını linker'a kaydet.
///
/// WASM modülü bu fonksiyonları `(import "env" "fn_name" ...)` ile çağırır.
pub fn register_host_functions(
    linker: &mut Linker<WasmiHostContext>,
) -> Result<(), WasmError> {
    // ── aetheros_log(ptr: i32, len: i32) ─────────────────
    // WASM tarafından mesaj logging için kullanılır.
    linker
        .func_wrap(
            "env",
            "aetheros_log",
            |mut caller: Caller<WasmiHostContext>, ptr: i32, len: i32| {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory());

                if let Some(mem) = mem {
                    let data = mem.data(&caller);
                    let start = ptr as usize;
                    let end = start + len as usize;
                    if end <= data.len() {
                        let msg = String::from_utf8_lossy(&data[start..end]);
                        tracing::debug!(target: "wasmi_host", "{}", msg);
                    }
                }
            },
        )
        .map_err(|e| WasmError::LinkerFailure { message: e.to_string() })?;

    // ── aetheros_now_ms() → i64 ───────────────────────────
    // UNIX timestamp milisaniye cinsinden döner.
    linker
        .func_wrap("env", "aetheros_now_ms", |_: Caller<WasmiHostContext>| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0)
        })
        .map_err(|e| WasmError::LinkerFailure { message: e.to_string() })?;

    Ok(())
}
