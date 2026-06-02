// ============================================================
// src/workflows/retries.rs
//
// Faz 5 Düzeltmesi: RetryPolicy Duplikasyonu Kaldırıldı
//
// ÖNCE:
//   Burada kendi RetryPolicy struct'ı vardı:
//     max_attempts: usize, backoff_ms: u64
//   task::retry::RetryPolicy ile neredeyse aynı ama
//   farklı alan isimleri ve eksik jitter/max_delay.
//
// SONRA:
//   task::retry::RetryPolicy re-export edildi.
//   Tek tanım, iki kullanım yeri. ✓
// ============================================================

pub use crate::task::retry::RetryPolicy as WorkflowRetryPolicy;
