// ============================================================
// src/task/retry.rs
//
// Faz 2 Eklemesi:
//
// [BUG #3] should_retry() eklendi.
//   Worker'ın tek karar noktası: bu hata + bu attempt
//   sayısıyla yeniden denenebilir mi?
//
//   Karar kriterleri (sırayla):
//   1. attempts >= max_attempts → hayır (limit doldu)
//   2. error Permanent sınıfta → hayır (tekrar denemek anlamsız)
//   3. Her ikisi de geçtiyse → evet, retry
// ============================================================

use std::time::Duration;

use rand::Rng;

use serde::{
    Deserialize,
    Serialize,
};

use crate::errors::wasm::WasmError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub jitter: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClassification {
    Retryable,
    Permanent,
}

impl RetryPolicy {
    /// Bu hata + mevcut attempt sayısıyla retry yapılmalı mı?
    ///
    /// `attempts`: şimdiye kadar yapılan deneme sayısı.
    ///   İlk denemeden sonra gelen ilk hata → attempts = 1.
    ///   max_attempts = 3 ise 3 kez denenip 3. başarısızsa retry yok.
    pub fn should_retry(
        &self,
        attempts: u32,
        error: &WasmError,
    ) -> bool {
        // Limit kontrolü önce — sınıflandırmaya gerek kalmaz
        if attempts >= self.max_attempts {
            return false;
        }

        // Kalıcı hata: tekrar denemek sonuç değiştirmez
        matches!(
            Self::classify_wasm_error(error),
            RetryClassification::Retryable
        )
    }

    pub fn classify_wasm_error(
        error: &WasmError,
    ) -> RetryClassification {
        match error {
            // Geçici durum — retry anlamlı
            WasmError::Timeout => RetryClassification::Retryable,
            WasmError::ExecutionPanic => RetryClassification::Retryable,
            WasmError::ExecutionFailure { .. } => RetryClassification::Retryable,

            // Kaynak sınırı aşımı — retry limit artırmaz,
            // ama farklı zamanda kaynak müsait olabilir
            WasmError::ResourceLimitExceeded => RetryClassification::Retryable,

            // Kalıcı hatalar — modül/config sorunu, retry'a gerek yok
            WasmError::InvalidModule => RetryClassification::Permanent,
            WasmError::InvalidConfiguration { .. } => RetryClassification::Permanent,
            WasmError::CapabilityDenied => RetryClassification::Permanent,
            WasmError::MissingEntrypoint => RetryClassification::Permanent,
            WasmError::SandboxViolation => RetryClassification::Permanent,
            WasmError::MemoryViolation => RetryClassification::Permanent,
            WasmError::EngineFailure { .. } => RetryClassification::Permanent,
            WasmError::LinkerFailure { .. } => RetryClassification::Permanent,
        }
    }

    /// Exponential backoff hesapla (jitter opsiyonel).
    ///
    /// `attempt`: kaçıncı denemeden sonra bekleniyor?
    ///   1. başarısızlık → attempt = 1 → base * 2^1
    ///   2. başarısızlık → attempt = 2 → base * 2^2
    ///   Overflow koruması: pow(attempt.min(16))
    pub fn next_delay(&self, attempt: u32) -> Duration {
        let multiplier = 2u64.pow(attempt.min(16));
        let delay_ms = self.base_delay_ms * multiplier;
        let capped = delay_ms.min(self.max_delay_ms);

        if self.jitter {
            let mut rng = rand::rng();
            let jitter = rng.random_range(0..1000);
            Duration::from_millis(capped + jitter)
        } else {
            Duration::from_millis(capped)
        }
    }
}
