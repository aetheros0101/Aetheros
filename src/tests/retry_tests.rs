// ============================================================
// src/tests/retry_tests.rs
//
// GRUP 2: Retry Mekanizması Testleri
//
// Test edilen özellikler:
//   - Her WasmError için doğru sınıflandırma
//   - should_retry(): attempts + error kombinasyonları
//   - max_attempts aşılınca retry yok
//   - next_delay(): exponential backoff doğruluğu
//   - Jitter: delay tutarlı aralıkta
// ============================================================

use crate::errors::wasm::WasmError;
use crate::task::retry::{
    RetryClassification,
    RetryPolicy,
};

// ── Yardımcı ──────────────────────────────────────────────

fn policy(max: u32) -> RetryPolicy {
    RetryPolicy {
        max_attempts: max,
        base_delay_ms: 100,
        max_delay_ms: 10_000,
        jitter: false,
    }
}

// ── Sınıflandırma testleri ────────────────────────────────

#[test]
fn timeout_is_retryable() {
    assert_eq!(
        RetryPolicy::classify_wasm_error(&WasmError::Timeout),
        RetryClassification::Retryable
    );
}

#[test]
fn execution_panic_is_retryable() {
    assert_eq!(
        RetryPolicy::classify_wasm_error(&WasmError::ExecutionPanic),
        RetryClassification::Retryable
    );
}

#[test]
fn resource_limit_is_retryable() {
    assert_eq!(
        RetryPolicy::classify_wasm_error(&WasmError::ResourceLimitExceeded),
        RetryClassification::Retryable
    );
}

#[test]
fn invalid_module_is_permanent() {
    assert_eq!(
        RetryPolicy::classify_wasm_error(&WasmError::InvalidModule {
            reason: "test".into(),
        }),
        RetryClassification::Permanent
    );
}

#[test]
fn invalid_configuration_is_permanent() {
    assert_eq!(
        RetryPolicy::classify_wasm_error(
            &WasmError::InvalidConfiguration {
                reason: "timeout_ms is 0".to_string(),
            }
        ),
        RetryClassification::Permanent
    );
}

#[test]
fn capability_denied_is_permanent() {
    assert_eq!(
        RetryPolicy::classify_wasm_error(&WasmError::CapabilityDenied),
        RetryClassification::Permanent
    );
}

#[test]
fn sandbox_violation_is_permanent() {
    assert_eq!(
        RetryPolicy::classify_wasm_error(&WasmError::SandboxViolation),
        RetryClassification::Permanent
    );
}

// ── should_retry() testleri ───────────────────────────────

/// BUG #3 regresyon: Retryable hata + kalan deneme → true
#[test]
fn should_retry_when_retryable_and_attempts_remain() {
    let p = policy(3);
    assert!(
        p.should_retry(1, &WasmError::Timeout),
        "1. başarısızlıktan sonra retry olmalı (max=3)"
    );
    assert!(
        p.should_retry(2, &WasmError::Timeout),
        "2. başarısızlıktan sonra retry olmalı (max=3)"
    );
}

/// max_attempts dolunca retry yok.
#[test]
fn no_retry_when_max_attempts_reached() {
    let p = policy(3);
    assert!(
        !p.should_retry(3, &WasmError::Timeout),
        "3. denemeden sonra retry olmamalı (max=3)"
    );
    assert!(
        !p.should_retry(10, &WasmError::Timeout),
        "10 denemede kesinlikle retry yok"
    );
}

/// Permanent hata → retry yok (attempts ne olursa olsun).
#[test]
fn no_retry_for_permanent_errors() {
    let p = policy(10);
    assert!(
        !p.should_retry(1, &WasmError::InvalidModule { reason: "test".into() }),
        "Permanent hata retry edilmemeli"
    );
    assert!(
        !p.should_retry(0, &WasmError::CapabilityDenied),
        "Permanent hata 0 denemede de retry edilmemeli"
    );
}

/// max_attempts = 1 → hiç retry yok.
#[test]
fn no_retry_when_max_attempts_is_one() {
    let p = policy(1);
    assert!(
        !p.should_retry(1, &WasmError::Timeout),
        "max_attempts=1 ile retry olmamalı"
    );
}

// ── next_delay() testleri ─────────────────────────────────

/// Exponential backoff: base * 2^attempt
#[test]
fn delay_exponential_backoff() {
    let p = policy(5);

    let d1 = p.next_delay(1).as_millis(); // 100 * 2 = 200
    let d2 = p.next_delay(2).as_millis(); // 100 * 4 = 400
    let d3 = p.next_delay(3).as_millis(); // 100 * 8 = 800

    assert_eq!(d1, 200, "1. deneme delay: 200ms");
    assert_eq!(d2, 400, "2. deneme delay: 400ms");
    assert_eq!(d3, 800, "3. deneme delay: 800ms");
}

/// max_delay_ms: delay asla aşmamalı.
#[test]
fn delay_capped_at_max() {
    let p = RetryPolicy {
        max_attempts: 10,
        base_delay_ms: 1000,
        max_delay_ms: 5000,
        jitter: false,
    };

    // 1000 * 2^10 = 1_024_000ms → cap: 5000ms
    let d = p.next_delay(10).as_millis();
    assert_eq!(d, 5000, "Delay max_delay_ms'i aşmamalı");
}

/// Jitter: delay 0..1000ms arasında fazladan ekleniyor.
#[test]
fn delay_with_jitter_in_range() {
    let p = RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 100,
        max_delay_ms: 60_000,
        jitter: true,
    };

    // base * 2^1 = 200, jitter 0-1000 → 200..1200
    let d = p.next_delay(1).as_millis();
    assert!(
        d >= 200 && d < 1200,
        "Jitter'lı delay 200-1200ms arasında olmalı, ama: {}ms", d
    );
}
