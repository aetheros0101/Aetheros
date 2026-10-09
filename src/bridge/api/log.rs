// ============================================================
// bridge/api/log.rs
//
// Log izleme API'si.
// ============================================================

use crate::bridge::api::error::require_runtime;
use crate::bridge::types::LogRecord;


/// Son `limit` kadar log entry döndür (yeniden eskiye sıralı).
///
/// LogScreen 2 saniyede bir bu fonksiyonu polling ile çeker.
/// limit: 0 → varsayılan 100.
pub fn get_recent_logs(limit: u32) -> Result<Vec<LogRecord>, String> {
    let rt = require_runtime()?;

    let limit = if limit == 0 { 100 } else { limit as usize };

    Ok(rt
        .log_buffer
        .recent(limit)
        .into_iter()
        .map(to_bridge_log_entry)
        .collect())
}

/// Belirli bir task'a ait log entry'leri döndür.
///
/// Task detay modalındaki "Loglar" sekmesi için.
/// limit: 0 → varsayılan 50.
pub fn get_task_logs(task_id: String, limit: u32) -> Result<Vec<LogRecord>, String> {
    let rt = require_runtime()?;

    let limit = if limit == 0 { 50 } else { limit as usize };

    Ok(rt
        .log_buffer
        .by_task(&task_id, limit)
        .into_iter()
        .map(to_bridge_log_entry)
        .collect())
}

/// logging::buffer::LogEntry (pub(crate), &'static str alanlı) →
/// bridge::types::LogRecord (pub, String alanlı, FRB-export edilebilir).
fn to_bridge_log_entry(e: crate::logging::buffer::LogEntry) -> LogRecord {
    LogRecord {
        timestamp_ms: e.timestamp_ms,
        level: e.level.to_string(),
        task_id: e.task_id,
        message: e.message,
        event_type: e.event_type.to_string(),
    }
}
