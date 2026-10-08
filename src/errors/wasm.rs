// ============================================================
// src/errors/wasm.rs
//
// Faz 3 Eklemesi:
//
// [BUG #8] InvalidConfiguration variant eklendi.
//   timeout_ms == 0 → ResourceLimitExceeded dönüyordu.
//   "Kaynak aşıldı" anlamsız — kaynak hiç kullanılmadı.
//   Doğru hata: "geçersiz yapılandırma".
//
//   Aynı zamanda RetryClassification'a etkisi:
//   InvalidConfiguration → Permanent (retry.rs'de eklendi)
//   Yanlış konfigürasyon retry'la düzelmez.
// ============================================================

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WasmError {
    #[error("execution timeout")]
    Timeout,

    #[error("execution panic")]
    ExecutionPanic,

    #[error("resource limit exceeded")]
    ResourceLimitExceeded,

    #[error("invalid module: {reason}")]
    InvalidModule { reason: String },

    /// [BUG #8] Yeni variant: timeout_ms == 0 gibi
    /// görev konfigürasyonu geçersizse bu döner.
    #[error("invalid task configuration: {reason}")]
    InvalidConfiguration { reason: String },

    #[error("capability denied")]
    CapabilityDenied,

    #[error("missing entrypoint")]
    MissingEntrypoint,

    #[error("sandbox violation")]
    SandboxViolation,

    #[error("memory violation")]
    MemoryViolation,

    #[error("engine failure: {message}")]
    EngineFailure { message: String },

    #[error("execution failure: {message}")]
    ExecutionFailure { message: String },

    #[error("linker failure: {message}")]
    LinkerFailure { message: String },
}
