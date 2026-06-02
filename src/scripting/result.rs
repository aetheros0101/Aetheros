// ============================================================
// src/scripting/result.rs
// ============================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptResult {
    pub script_name: String,
    /// Ham WASM çıktısı (ileride UTF-8 decode edilir).
    pub output: Vec<u8>,
    pub elapsed_ms: u64,
    pub executed_at: DateTime<Utc>,
}

impl ScriptResult {
    /// Çıktıyı UTF-8 string olarak al.
    /// Geçersiz byte'lar lossy replace ile işlenir.
    pub fn output_as_string(&self) -> String {
        String::from_utf8_lossy(&self.output).into_owned()
    }

    pub fn is_empty(&self) -> bool {
        self.output.is_empty()
    }
}
