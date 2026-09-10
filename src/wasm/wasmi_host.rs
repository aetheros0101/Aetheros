// src/wasm/wasmi_host.rs
// wasmi backend için host fonksiyon tanımları (Android/embedded)

use wasmi::{Caller, Linker};

use crate::errors::wasm::WasmError;

pub struct WasmiHostContext {
    pub fuel_limit: u64,
    pub output:     Vec<u8>,
}

impl WasmiHostContext {
    pub fn new(fuel_limit: u64) -> Self {
        Self { fuel_limit, output: Vec::new() }
    }
}

pub fn register_host_functions(
    linker: &mut Linker<WasmiHostContext>,
) -> Result<(), WasmError> {
    // aetheros_log(ptr, len) — WASM'dan log mesajı
    linker
        .func_wrap(
            "env",
            "aetheros_log",
            |caller: Caller<WasmiHostContext>, ptr: i32, len: i32| {
                let mem = caller.get_export("memory")
                    .and_then(|e| e.into_memory());
                if let Some(mem) = mem {
                    let data = mem.data(&caller);
                    let start = ptr as usize;
                    let end   = (start + len as usize).min(data.len());
                    let msg   = String::from_utf8_lossy(&data[start..end]);
                    tracing::debug!(target: "wasmi_host", "{}", msg);
                }
            },
        )
        .map_err(|e| WasmError::LinkerFailure { message: e.to_string() })?;

    // aetheros_now_ms() → i64 — UNIX timestamp ms
    linker
        .func_wrap(
            "env",
            "aetheros_now_ms",
            |_: Caller<WasmiHostContext>| -> i64 {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0)
            },
        )
        .map_err(|e| WasmError::LinkerFailure { message: e.to_string() })?;

    Ok(())
}
