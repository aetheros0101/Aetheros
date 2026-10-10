// ============================================================
// bridge/api/metrics.rs
//
// Metrik görüntüsü API'si.
// ============================================================

use crate::bridge::api::error::require_runtime;
use crate::bridge::types::MetricsSnapshot;

// ── Metrikler ─────────────────────────────────────────────

/// Anlık metrik görüntüsü al.
pub async fn get_metrics() -> Result<MetricsSnapshot, String> {
    let rt = require_runtime()?;

    let snap = rt.metrics.snapshot();

    Ok(MetricsSnapshot {
        active_workers: snap.active_workers,
        queued_tasks: snap.queued_tasks,
        completed_tasks: snap.completed_tasks,
        failed_tasks: snap.failed_tasks,
        retried_tasks: snap.retried_tasks,
    })
}
