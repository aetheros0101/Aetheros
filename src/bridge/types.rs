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

// ── Log entry (Dart'a açık) ───────────────────────────────
//
// logging::buffer::LogEntry'nin FRB-uyumlu karşılığı.
// Adı bilerek "LogEntry" DEĞİL "LogRecord" — FRB tip taraması
// bare-name (tam path'siz) çalışıyor; iki ayrı modülde aynı
// isimli tip olunca (bridge::types::LogEntry vs
// logging::buffer::LogEntry) FRB rastgele/yanlış olanını
// SseEncode ile bağlıyor ve Dart tarafı kırılıyor. Bu yüzden
// burada kasıtlı olarak farklı bir isim kullanılıyor.

#[frb(dart_metadata = ("freezed"))]
pub struct LogRecord {
    /// Unix ms
    pub timestamp_ms: i64,
    /// "INFO" | "WARN" | "ERROR"
    pub level: String,
    /// Varsa task UUID
    pub task_id: Option<String>,
    /// Türkçe kısa açıklama
    pub message: String,
    /// "TaskStarted" | "TaskFailed" | ...
    pub event_type: String,
}

// ── Agent tipleri (FRB bridge) ────────────────────────────────

#[frb(dart_metadata = ("freezed"))]
pub struct AgentStartResponse {
    pub execution_id: String,
    pub agent_id:     String,
    pub status:       String,
}

#[frb(dart_metadata = ("freezed"))]
pub struct AgentStatusResponse {
    pub execution_id: String,
    pub agent_id:     String,
    pub objective:    String,
    pub status:       String,
    pub error:        Option<String>,
    /// Unix ms
    pub started_at:   i64,
    pub finished_at:  Option<i64>,
    /// Planner'ın ürettiği adım isimleri (boş = henüz plan yok / running).
    pub planned_steps: Vec<String>,
}

// ── Workflow tipleri (FRB bridge) ─────────────────────────────────────────────

#[frb(dart_metadata = ("freezed"))]
pub struct WorkflowStartResponse {
    pub workflow_id: String,
    pub name:        String,
    pub status:      String,
}

#[frb(dart_metadata = ("freezed"))]
pub struct WorkflowStatusResponse {
    pub workflow_id: String,
    pub name:        String,
    pub status:      String,
    pub error:       Option<String>,
    /// Unix ms
    pub started_at:  i64,
    pub finished_at: Option<i64>,
}

// ── Cluster tipleri (FRB bridge) ──────────────────────────────

#[frb(dart_metadata = ("freezed"))]
pub struct ClusterNodeResponse {
    pub node_id:           String,
    pub address:           String,
    pub healthy:           bool,
    pub capabilities:      Vec<String>,
    pub cpu_percent:       f32,
    pub memory_mb:         u64,
    pub active_executions: u64,
}

#[frb(dart_metadata = ("freezed"))]
pub struct ClusterStatusResponse {
    pub health:     String,
    pub total:      u64,
    pub healthy:    u64,
    pub has_quorum: bool,
    pub leader:     Option<String>,
    pub nodes:      Vec<ClusterNodeResponse>,
}

#[frb(dart_metadata = ("freezed"))]
pub struct NodeRegistrationResponse {
    pub node_id: String,
    pub address: String,
    pub status:  String,
}
