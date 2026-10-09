// ============================================================
// bridge/api/helpers.rs
//
// Ortak parse ve format yardımcıları.
// Tekrarlanan dönüşüm mantığı burada toplanır.
// ============================================================

use crate::task::priority::TaskPriority;

/// priority string → TaskPriority
pub(crate) fn parse_priority(s: &str) -> Result<TaskPriority, String> {
    match s.to_lowercase().as_str() {
        "critical" => Ok(TaskPriority::Critical),
        "high" => Ok(TaskPriority::High),
        "normal" => Ok(TaskPriority::Normal),
        "low" => Ok(TaskPriority::Low),
        other => Err(format!("Geçersiz priority: '{other}'")),
    }
}

/// hex string → [u8; 32]. Boş string → sıfır hash (modülsüz task).
pub(crate) fn parse_hash(hex_str: &str) -> Result<[u8; 32], String> {
    if hex_str.is_empty() {
        return Ok([0u8; 32]); // Modülsüz task
    }

    let bytes = hex::decode(hex_str).map_err(|_| format!("Geçersiz hex hash: '{hex_str}'"))?;

    if bytes.len() != 32 {
        return Err(format!("Hash 32 byte olmalı, {} byte geldi", bytes.len()));
    }

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);
    Ok(hash)
}

/// Capability adını (Flutter/FRB'den gelen string) enum'a çevirir.
/// `grant_agent_capability` ve `start_agent` aynı tabloyu kullanır.
pub(crate) fn parse_capability(
    name: &str,
) -> Result<crate::agents::capabilities::AgentCapability, String> {
    use crate::agents::capabilities::AgentCapability as C;
    match name {
        "wasm_execution" => Ok(C::WasmExecution),
        "workflow_execution" => Ok(C::WorkflowExecution),
        "ai_reasoning" => Ok(C::AiReasoning),
        "remote_execution" => Ok(C::RemoteExecution),
        "terminal_execution" => Ok(C::TerminalExecution),
        "workspace_read" => Ok(C::WorkspaceRead),
        "workspace_write" => Ok(C::WorkspaceWrite),
        "workspace_mutate" => Ok(C::WorkspaceMutate),
        other => Err(format!("Bilinmeyen capability: '{other}'")),
    }
}

/// Özet satırı için argümanları ` [a b c]` biçiminde yazar (boşsa "").
/// Audit'e yazılırken zaten maskelenmiş/kırpılmış olarak gelir.
pub(crate) fn fmt_call_args(arguments: &[String]) -> String {
    if arguments.is_empty() {
        String::new()
    } else {
        format!(" [{}]", arguments.join(" "))
    }
}

/// `AuditEventKind`'ı ("governor_decision" gibi) kısa bir etikete ve
/// tek satırlık okunabilir bir özete çevirir.
pub(crate) fn describe_audit_event(
    kind: &crate::logging::audit::AuditEventKind,
) -> (&'static str, String) {
    use crate::logging::audit::AuditEventKind as K;
    match kind {
        K::GovernorDecision {
            tool_name,
            decision,
            reason,
            arguments,
        } => (
            "governor_decision",
            match reason {
                Some(r) => format!(
                    "'{tool_name}'{} → {decision} ({r})",
                    fmt_call_args(arguments)
                ),
                None => format!("'{tool_name}'{} → {decision}", fmt_call_args(arguments)),
            },
        ),
        K::ToolInvoked {
            tool_name,
            success,
            error,
            arguments,
            ..
        } => (
            "tool_invoked",
            if *success {
                format!("'{tool_name}'{} çalıştı", fmt_call_args(arguments))
            } else {
                format!(
                    "'{tool_name}'{} başarısız: {}",
                    fmt_call_args(arguments),
                    error.clone().unwrap_or_default()
                )
            },
        ),
        K::ExecutionPaused {
            tool_name,
            reason,
            arguments,
            ..
        } => (
            "execution_paused",
            format!(
                "'{tool_name}'{} onay bekliyor: {reason}",
                fmt_call_args(arguments)
            ),
        ),
        K::ExecutionResumed { approval_id } => (
            "execution_resumed",
            format!("onay {approval_id} ile devam edildi"),
        ),
        K::ApprovalDenied { reason, .. } => {
            ("approval_denied", format!("kullanıcı reddetti: {reason}"))
        }
        K::ExecutionCompleted => ("execution_completed", "tamamlandı".to_string()),
        K::ExecutionFailed { error } => ("execution_failed", format!("hata: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_priority_valid() {
        assert!(matches!(
            parse_priority("critical"),
            Ok(TaskPriority::Critical)
        ));
        assert!(matches!(parse_priority("HIGH"), Ok(TaskPriority::High)));
        assert!(matches!(parse_priority("normal"), Ok(TaskPriority::Normal)));
        assert!(matches!(parse_priority("low"), Ok(TaskPriority::Low)));
    }

    #[test]
    fn parse_priority_invalid() {
        assert!(parse_priority("medium").is_err());
        assert!(parse_priority("").is_err());
    }

    #[test]
    fn parse_hash_empty() {
        assert_eq!(parse_hash("").unwrap(), [0u8; 32]);
    }

    #[test]
    fn parse_hash_valid() {
        let hex = "00".repeat(32);
        let h = parse_hash(&hex).unwrap();
        assert_eq!(h, [0u8; 32]);
    }

    #[test]
    fn parse_hash_invalid_len() {
        assert!(parse_hash("abcd").is_err());
    }

    #[test]
    fn parse_capability_known() {
        assert!(parse_capability("wasm_execution").is_ok());
        assert!(parse_capability("workspace_read").is_ok());
    }

    #[test]
    fn parse_capability_unknown() {
        assert!(parse_capability("fly").is_err());
    }

    #[test]
    fn fmt_call_args_empty() {
        assert_eq!(fmt_call_args(&[]), "");
    }

    #[test]
    fn fmt_call_args_some() {
        assert_eq!(fmt_call_args(&["a".into(), "b".into()]), " [a b]");
    }
}
