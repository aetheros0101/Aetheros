//! İptal sinyali: graceful vs immediate.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancellationMode {
    /// Mevcut adımı bitir, yenisine başlama.
    Graceful,
    /// Mümkün olan en kısa sürede dur.
    Immediate,
}

/// Paylaşılan iptal bayrağı (runtime adım döngüsünde kontrol edilir).
#[derive(Clone, Default)]
pub struct CancellationToken {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    cancelled: AtomicBool,
    immediate: AtomicBool,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self, mode: CancellationMode) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
        if mode == CancellationMode::Immediate {
            self.inner.immediate.store(true, Ordering::SeqCst);
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    pub fn is_immediate(&self) -> bool {
        self.inner.immediate.load(Ordering::SeqCst)
    }

    pub fn mode(&self) -> Option<CancellationMode> {
        if !self.is_cancelled() {
            None
        } else if self.is_immediate() {
            Some(CancellationMode::Immediate)
        } else {
            Some(CancellationMode::Graceful)
        }
    }

    pub fn reset(&self) {
        self.inner.cancelled.store(false, Ordering::SeqCst);
        self.inner.immediate.store(false, Ordering::SeqCst);
    }
}

/// Eski enum adı ile uyumluluk.
pub type AgentCancellation = CancellationMode;
