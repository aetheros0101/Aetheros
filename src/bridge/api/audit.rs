// ============================================================
// bridge/api/audit.rs
//
// Audit log listeleme.
// ============================================================

use crate::bridge::api::error::require_runtime;
use crate::bridge::api::helpers::describe_audit_event;
use crate::bridge::types::AuditEventResponse;

/// Denetim kayıtlarını listele. `execution_id` verilirse sadece o
/// execution'a ait kayıtlar döner, verilmezse hepsi (en yeni en sonda).
pub fn list_audit_events(execution_id: Option<String>) -> Result<Vec<AuditEventResponse>, String> {
    let rt = require_runtime()?;

    let events = match execution_id {
        Some(hex) => {
            let id =
                uuid::Uuid::parse_str(&hex).map_err(|e| format!("Geçersiz execution_id: {e}"))?;
            rt.audit_log.list_for_execution(id)
        }
        None => rt.audit_log.list(),
    };

    Ok(events
        .into_iter()
        .map(|e| {
            let (kind_label, summary) = describe_audit_event(&e.kind);
            AuditEventResponse {
                id: e.id.to_string(),
                agent_id: e.agent_id.to_string(),
                execution_id: e.execution_id.to_string(),
                kind_label: kind_label.to_string(),
                summary,
                details_json: serde_json::to_string(&e.kind).unwrap_or_default(),
                created_at: e.timestamp.timestamp_millis(),
            }
        })
        .collect())
}
