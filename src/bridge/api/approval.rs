// ============================================================
// bridge/api/approval.rs
//
// Approval Engine: bekleyen onaylar ve cevap.
// ============================================================

use tracing::info;

use crate::bridge::api::error::{describe_runtime_error, require_runtime};
use crate::bridge::api::terminal::append_workspace_tools;
use crate::bridge::types::PendingApprovalResponse;

// ── V10 Sprint 5: Approval Engine ──────────────────────────
//
// SecurityGovernor bir ToolCall için RequiresApproval dediğinde,
// AgentRuntime execution'ı invoke etmeden DURDURUP bir PendingApproval
// kaydı bırakır (bkz. agents::approval). Bu iki fonksiyon, "Bekleyen
// Onaylar" ekranının ihtiyaç duyduğu tüm akışı sağlar.

/// Onay bekleyen tüm execution'ları listele.
pub fn list_pending_approvals() -> Result<Vec<PendingApprovalResponse>, String> {
    let rt = require_runtime()?;

    // V10 B3: süresi dolmuş onaylar listede görünmesin (ve verilemesin).
    crate::bridge::state::expire_stale_approvals(
        &rt.approval_store,
        &rt.audit_log,
        &rt.agent_registry,
    );

    Ok(rt
        .approval_store
        .list()
        .into_iter()
        .map(|p| PendingApprovalResponse {
            id: p.id.to_string(),
            execution_id: p.context.execution_id.to_string(),
            agent_id: p.context.agent_id.to_string(),
            objective: p.objective,
            tool_name: p.tool_call.tool_name,
            arguments: p.tool_call.arguments,
            reason: p.reason,
            created_at: p.created_at.timestamp_millis(),
        })
        .collect())
}

/// Bekleyen bir onaya cevap ver.
///
/// `approved = true`  → onaylanan tool_call çalıştırılır, execution
///                       kalan adımlarla arka planda devam eder.
/// `approved = false` → execution kalıcı olarak reddedilmiş sayılır,
///                       hiçbir şey invoke edilmez.
pub async fn respond_to_approval(approval_id: String, approved: bool) -> Result<(), String> {
    let rt = require_runtime()?;

    let id =
        uuid::Uuid::parse_str(&approval_id).map_err(|e| format!("Geçersiz approval_id: {e}"))?;

    // V10 B3: TTL'i dolmuş onay verilemez — önce temizle.
    crate::bridge::state::expire_stale_approvals(
        &rt.approval_store,
        &rt.audit_log,
        &rt.agent_registry,
    );

    // take(): kaydı çıkarır — aynı onaya iki kere cevap verilemez.
    let pending = rt.approval_store.take(&id).ok_or_else(|| {
        format!(
            "Onay kaydı bulunamadı (zaten işlenmiş ya da süresi dolmuş olabilir): {approval_id}"
        )
    })?;

    let execution_id = pending.context.execution_id;

    if !approved {
        rt.audit_log.record(
            pending.context.agent_id,
            execution_id,
            crate::logging::audit::AuditEventKind::ApprovalDenied {
                approval_id: id,
                reason: pending.reason.clone(),
            },
        );
        if let Some(mut entry) = rt.agent_registry.get_mut(&execution_id) {
            entry.status = "denied".into();
            entry.error = Some(format!("Kullanıcı onayı reddetti: {}", pending.reason));
            entry.finished_at = Some(chrono::Utc::now());
            entry.pending_approval_id = None;
        }
        info!(approval_id = %id, execution_id = %execution_id, "Approval denied by user");
        return Ok(());
    }

    if let Some(mut entry) = rt.agent_registry.get_mut(&execution_id) {
        entry.status = "running".into();
        entry.pending_approval_id = None;
    }

    // Resume anında GÜNCEL registry'den taze tool listesi kur —
    // start_agent'taki ile aynı desen (bkz. yukarısı).
    let mut tools: Vec<std::sync::Arc<dyn crate::types::agent_tool::AgentTool>> = rt
        .script_registry
        .list()
        .into_iter()
        .map(|def| {
            std::sync::Arc::new(crate::scripting::tool::ScriptTool::new(
                def,
                rt.script_engine.clone(),
            )) as std::sync::Arc<dyn crate::types::agent_tool::AgentTool>
        })
        .collect();
    tools.push(std::sync::Arc::new(
        crate::agents::terminal_tool::TerminalAgentTool::with_workspace(std::path::PathBuf::from(
            &rt.workspace_dir,
        )),
    ));
    append_workspace_tools(rt, &mut tools);

    let ai_router = rt.ai_router.clone();
    let capability_engine = rt.capability_engine.clone();
    let risk_engine = rt.risk_engine.clone();
    let approval_store = rt.approval_store.clone();
    let audit_log = rt.audit_log.clone();
    let registry = rt.agent_registry.clone();

    info!(approval_id = %id, execution_id = %execution_id, "Approval granted by user — resuming execution");

    tokio::spawn(async move {
        let result = crate::agents::runtime::AgentRuntime::resume(
            pending,
            tools,
            Some(ai_router),
            Some(capability_engine),
            Some(risk_engine),
            Some(approval_store),
            Some(audit_log),
        )
        .await;
        let finished_at = chrono::Utc::now();
        if let Some(mut entry) = registry.get_mut(&execution_id) {
            match result {
                Ok(crate::agents::runtime::AgentOutcome::Completed) => {
                    entry.status = "completed".into();
                    entry.finished_at = Some(finished_at);
                }
                Ok(crate::agents::runtime::AgentOutcome::PendingApproval { approval_id }) => {
                    entry.status = "pending_approval".into();
                    entry.pending_approval_id = Some(approval_id);
                }
                Err(e) => {
                    entry.status = "failed".into();
                    entry.error = Some(describe_runtime_error(&e));
                    entry.finished_at = Some(finished_at);
                }
            }
        }
    });

    Ok(())
}
