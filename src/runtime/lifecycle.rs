// ============================================================
// src/runtime/lifecycle.rs
//
// Faz 1 Eklemesi:
//
// from_u8() metodu eklendi.
//
// Runtime, AtomicU8 üzerinden state tuttuğu için u8 → enum
// dönüşümü gereklidir. Orijinalde bu dönüşüm runtime.rs içinde
// ham match ile yapılıyordu (magic number'lar dağınık). Merkezi
// from_u8() ile:
//   - Tek doğru kaynak burası
//   - Yeni state eklenince sadece burası güncellenir
//   - Geçersiz değer → Failed (panic yerine güvenli fallback)
// ============================================================

use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
)]
pub enum RuntimeState {
    Created,   // 0
    Starting,  // 1
    Running,   // 2
    Draining,  // 3
    Stopping,  // 4
    Stopped,   // 5
    Failed,    // 6 — ve bilinmeyen tüm değerler
}

impl RuntimeState {
    /// AtomicU8 değerini enum'a dönüştür.
    ///
    /// Bilinmeyen bir değer gelirse `Failed` döner —
    /// panic etmez, observable bir hata state'i üretir.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Created,
            1 => Self::Starting,
            2 => Self::Running,
            3 => Self::Draining,
            4 => Self::Stopping,
            5 => Self::Stopped,
            _ => Self::Failed,
        }
    }

    /// State'in terminal (nihai) olup olmadığı.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Stopped | Self::Failed)
    }

    /// Dışarıya "hazır" sinyali verilip verilmeyeceği.
    pub fn is_running(self) -> bool {
        matches!(self, Self::Running | Self::Draining)
    }
}
