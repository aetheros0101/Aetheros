// ============================================================
// src/bridge/types.rs
//
// Flutter ↔ Rust arasındaki veri tipleri.
//
// NEDEN ayrı tipler?
//   TaskDefinition içinde Timestamp, RetryPolicy, wasm_module_hash
//   gibi Rust-spesifik tipler var. FRB bunları Dart'a çeviremez
//   (özellikle [u8;32] array ve chrono::DateTime).
//
//   Bu tipler:
//   - Dart'ın anlayacağı String/i64/bool/u32 kullanır
//   - Flutter UI doğrudan bu tipleri render eder
//   - Rust iç tiplerinden bağımsız evrilebilir
//
// FRB, pub struct alanlarını otomatik Dart sınıfına çevirir.
// Tüm alanlar public olmalı.
// ============================================================

use flutter_rust_bridge::frb;

// ── Task gönderme isteği ──────────────────────────────────

/// Flutter'ın task göndermek için kullandığı tip.
///
/// wasm_module_hash: 64 karakter hex string (SHA-256)
/// Boş string → modülsüz task (test için)
#[frb(dart_metadata = ("freezed"))]
pub struct TaskRequest {
    /// WASM modülünün SHA-256 hash'i (hex, 64 karakter)
    /// Boş: "" → modülsüz task
    pub wasm_module_hash: String,

    /// Modül içindeki fonksiyon adı
    pub entrypoint: String,

    /// "critical" | "high" | "normal" | "low"
    pub priority: String,

    /// Milisaniye cinsinden timeout (0 = 30s varsayılan)
    pub timeout_ms: u64,

    /// Yeniden deneme sayısı
    pub max_retries: u32,
}

// ── Task durum yanıtı ─────────────────────────────────────

/// Flutter'ın task durumunu göstermek için kullandığı tip.
#[frb(dart_metadata = ("freezed"))]
pub struct TaskStatusResponse {
    pub task_id: String,

    /// "Created"|"Queued"|"Executing"|"Completed"|"Failed"|"Cancelled"|"Retrying"
    pub state: String,

    pub created_at: i64,  // Unix timestamp (ms)
    pub updated_at: i64,

    /// Kaç kez denendi
    pub attempts: u32,

    /// Hata varsa açıklaması
    pub error_message: Option<String>,
}

// ── Metrik anlık görüntüsü ────────────────────────────────
// metrics::runtime::MetricsSnapshot ile aynı alanlar —
// pub use ile tek tip kalır, FRB SseEncode impl çakışmaz.
pub use crate::metrics::runtime::MetricsSnapshot;

// ── Runtime bilgisi ───────────────────────────────────────

#[frb(dart_metadata = ("freezed"))]
pub struct RuntimeInfo {
    pub version:      String,
    pub is_running:   bool,
    pub backend:      String,   // "wasmtime" | "wasmi"
    pub worker_count: u32,
}

// ── Modül yükleme yanıtı ─────────────────────────────────

#[frb(dart_metadata = ("freezed"))]
pub struct ModuleUploadResponse {
    /// Başarılı SHA-256 hash (hex)
    pub hash: String,
    /// Bayt cinsinden modül boyutu
    pub size: u64,
}

// ── Log entry ─────────────────────────────────────────────

/// Tek bir log satırı.
///
/// EventBus olaylarından üretilir, `get_recent_logs()` ile çekilir.
/// Flutter tarafı bunu LogScreen'de render eder.
#[frb(dart_metadata = ("freezed"))]
pub struct LogEntry {
    /// Unix milisaniye — `DateTime.fromMillisecondsSinceEpoch()`
    pub timestamp_ms: i64,

    /// "INFO" | "WARN" | "ERROR"
    pub level: String,

    /// Varsa ilgili task'ın UUID'si
    pub task_id: Option<String>,

    /// Kullanıcıya gösterilecek kısa mesaj (Türkçe)
    pub message: String,

    /// İç olay tipi — "TaskStarted" | "TaskFailed" | ...
    /// Filtre ve debug için kullanılır.
    pub event_type: String,
}
