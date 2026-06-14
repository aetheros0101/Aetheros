// ============================================================
// src/logging/buffer.rs  — Sprint 7: Log izleme
//
// TASARIM:
//   Uygulama içi dairesel log tamponu.
//   EventBus'tan gelen SystemEvent'ler burada LogEntry'ye
//   dönüştürülür ve Flutter'a bridge üzerinden sunulur.
//
// KAPASİTE:
//   MAX_ENTRIES = 500 → ~40–60 KB RAM (tahmin)
//   Dolu olduğunda en eski entry silinir (VecDeque pop_front).
//
// THREAD GÜVENLİĞİ:
//   Arc<Mutex<VecDeque>> — tokio task (yazar) + bridge calls (okuyucu)
//   Mutex tercih edildi: lock süresi çok kısa (push/drain).
//
// KULLANIM:
//   init_mobile_runtime() → LogBuffer::new() oluşturur
//   tokio::spawn(log_collector()) → EventBus'a subscribe, push
//   bridge: get_recent_logs() / get_task_logs() → drain
// ============================================================

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::events::bus::{EventBus, SystemEvent};
use crate::events::task::TaskEvent;

const MAX_ENTRIES: usize = 500;

// ── Log entry ─────────────────────────────────────────────
//
// pub(crate) — FRB bu struct'ı görmemeli.
// Dart'a açık LogEntry bridge::types::LogEntry'dir.
// İkisi aynı ismi taşırsa FRB rastgele birini seçer ve
// &'static str lifetime'ı nedeniyle binding üretemez →
// getRecentLogs/getTaskLogs Dart tarafında kaybolur.

#[derive(Debug, Clone)]
pub(crate) struct LogEntry {
    /// Unix ms
    pub(crate) timestamp_ms: i64,
    /// "INFO" | "WARN" | "ERROR"
    pub(crate) level: &'static str,
    /// Varsa task UUID
    pub(crate) task_id: Option<String>,
    /// Türkçe kısa açıklama
    pub(crate) message: String,
    /// "TaskStarted" | "TaskFailed" | ...
    pub(crate) event_type: &'static str,
}

impl LogEntry {
    fn now_ms() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }

    fn task(
        level: &'static str,
        task_id: String,
        message: String,
        event_type: &'static str,
    ) -> Self {
        Self {
            timestamp_ms: Self::now_ms(),
            level,
            task_id: Some(task_id),
            message,
            event_type,
        }
    }

    fn system(level: &'static str, message: String, event_type: &'static str) -> Self {
        Self {
            timestamp_ms: Self::now_ms(),
            level,
            task_id: None,
            message,
            event_type,
        }
    }
}

// ── Buffer ────────────────────────────────────────────────

#[derive(Clone)]
pub struct LogBuffer {
    inner: Arc<Mutex<VecDeque<LogEntry>>>,
}

impl LogBuffer {
    // new/clear/len: LogEntry'ye dokunmuyor → pub kalabilir
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_ENTRIES))),
        }
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    // push/recent/by_task: pub(crate) LogEntry alıyor/döndürüyor.
    //
    // Rust E0446: "restricted type `LogEntry` in public interface"
    // → pub fn'in parametresi veya dönüş tipi pub(crate) olamaz.
    // Çözüm: bu metodları da pub(crate) yap.
    // bridge/api.rs aynı crate içinde olduğu için erişim sorunsuz.

    /// Log entry ekle. Kapasite doluysa en eski silinir.
    pub(crate) fn push(&self, entry: LogEntry) {
        let mut buf = self.inner.lock().unwrap();
        if buf.len() >= MAX_ENTRIES {
            buf.pop_front();
        }
        buf.push_back(entry);
    }

    /// Son `limit` kadar entry döndür (yeniden eskiye sıralı).
    pub(crate) fn recent(&self, limit: usize) -> Vec<LogEntry> {
        let buf = self.inner.lock().unwrap();
        buf.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Belirli task_id'ye ait entry'leri döndür.
    pub(crate) fn by_task(&self, task_id: &str, limit: usize) -> Vec<LogEntry> {
        let buf = self.inner.lock().unwrap();
        buf.iter()
            .rev()
            .filter(|e| e.task_id.as_deref() == Some(task_id))
            .take(limit)
            .cloned()
            .collect()
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Collector task ────────────────────────────────────────
//
// init_mobile_runtime() bu fonksiyonu spawn eder.
// EventBus'tan tüm SystemEvent'leri alır, LogEntry'ye çevirir,
// LogBuffer'a push eder.

pub async fn log_collector(bus: EventBus, buffer: LogBuffer) {
    let mut rx = bus.subscribe();

    // Runtime başlangıç log'u
    buffer.push(LogEntry::system(
        "INFO",
        "AetherOS runtime başlatıldı".into(),
        "RuntimeStarted",
    ));

    loop {
        match rx.recv().await {
            Ok(event) => {
                if let Some(entry) = event_to_entry(event) {
                    buffer.push(entry);
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                buffer.push(LogEntry::system(
                    "WARN",
                    format!("Log tamponu {n} event kaçırdı"),
                    "BufferLag",
                ));
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                // EventBus kapandı → runtime shutdown
                break;
            }
        }
    }
}

// ── Event → LogEntry dönüşümü ─────────────────────────────

fn event_to_entry(event: SystemEvent) -> Option<LogEntry> {
    match event {
        SystemEvent::Task(task_event) => Some(task_event_to_entry(task_event)),
        SystemEvent::Worker(_) => None, // Worker internal event'leri gösterme
        SystemEvent::Runtime(_) => None,
        SystemEvent::Telemetry(_) => None,
    }
}

fn task_event_to_entry(event: TaskEvent) -> LogEntry {
    match event {
        TaskEvent::TaskQueued { task_id } => LogEntry::task(
            "INFO",
            task_id.0.to_string(),
            "Kuyruğa eklendi".into(),
            "TaskQueued",
        ),
        TaskEvent::TaskStarted { task_id } => LogEntry::task(
            "INFO",
            task_id.0.to_string(),
            "Çalıştırılıyor...".into(),
            "TaskStarted",
        ),
        TaskEvent::TaskCompleted { task_id } => LogEntry::task(
            "INFO",
            task_id.0.to_string(),
            "Tamamlandı ✓".into(),
            "TaskCompleted",
        ),
        TaskEvent::TaskFailed { task_id } => LogEntry::task(
            "ERROR",
            task_id.0.to_string(),
            "Başarısız ✗".into(),
            "TaskFailed",
        ),
        TaskEvent::TaskCancelled { task_id } => LogEntry::task(
            "WARN",
            task_id.0.to_string(),
            "İptal edildi".into(),
            "TaskCancelled",
        ),
        TaskEvent::TaskRetrying { task_id, attempt } => LogEntry::task(
            "WARN",
            task_id.0.to_string(),
            format!("Yeniden deneniyor (deneme {attempt})"),
            "TaskRetrying",
        ),
        TaskEvent::TaskRetried { task_id, attempt } => LogEntry::task(
            "INFO",
            task_id.0.to_string(),
            format!("Yeniden deneme {attempt} gönderildi"),
            "TaskRetried",
        ),
    }
}
