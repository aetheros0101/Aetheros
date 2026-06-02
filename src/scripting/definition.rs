// ============================================================
// src/scripting/definition.rs
//
// Kullanıcının tanımladığı script yapısı.
//
// ScriptDefinition iki şekilde oluşturulur:
//   1. from_wat()   → WebAssembly Text Format string'den
//   2. from_binary()→ Derlenmiş WASM binary'den (hex veya raw)
//
// WAT → WASM dönüşümü wasmtime::wat::parse_str() ile yapılır.
// Bu sayede kullanıcı düz metin script yazabilir.
// ============================================================

use serde::{
    Deserialize,
    Serialize,
};

use crate::errors::wasm::WasmError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptDefinition {
    /// Script'in unique adı (registry'de anahtar).
    pub name: String,

    /// Script açıklaması (opsiyonel).
    pub description: Option<String>,

    /// Çalıştırılacak WASM binary.
    pub wasm_binary: Vec<u8>,

    /// WASM modülündeki fonksiyon adı.
    pub entrypoint: String,

    /// Maksimum çalışma süresi (ms).
    pub timeout_ms: u64,
}

impl ScriptDefinition {
    /// WAT (WebAssembly Text Format) stringinden script oluştur.
    ///
    /// Kullanıcı doğrudan metin script yazabilir:
    /// ```wat
    /// (module
    ///   (func (export "main"))
    /// )
    /// ```
    pub fn from_wat(
        name: impl Into<String>,
        wat_source: &str,
        entrypoint: impl Into<String>,
        timeout_ms: u64,
    ) -> Result<Self, WasmError> {
        let wasm_binary =
            wat::parse_str(wat_source).map_err(|_e| {
                WasmError::InvalidModule
            })?;

        Ok(Self {
            name: name.into(),
            description: None,
            wasm_binary,
            entrypoint: entrypoint.into(),
            timeout_ms,
        })
    }

    /// Derlenmiş WASM binary'den script oluştur.
    pub fn from_binary(
        name: impl Into<String>,
        wasm_binary: Vec<u8>,
        entrypoint: impl Into<String>,
        timeout_ms: u64,
    ) -> Self {
        Self {
            name: name.into(),
            description: None,
            wasm_binary,
            entrypoint: entrypoint.into(),
            timeout_ms,
        }
    }

    /// Hex string'den binary'ye çevirerek oluştur.
    /// REST API üzerinden hex olarak gelen script'ler için.
    pub fn from_hex(
        name: impl Into<String>,
        hex_str: &str,
        entrypoint: impl Into<String>,
        timeout_ms: u64,
    ) -> Result<Self, WasmError> {
        let wasm_binary = hex::decode(hex_str)
            .map_err(|_e| WasmError::InvalidModule)?;

        Ok(Self::from_binary(
            name,
            wasm_binary,
            entrypoint,
            timeout_ms,
        ))
    }

    pub fn with_description(
        mut self,
        desc: impl Into<String>,
    ) -> Self {
        self.description = Some(desc.into());
        self
    }
}
