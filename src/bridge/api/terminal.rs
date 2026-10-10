// ============================================================
// bridge/api/terminal.rs
//
// Kullanıcı terminali: komut kontrolü ve çalıştırma.
// ============================================================

use crate::bridge::api::error::require_runtime;
use crate::bridge::types::{TerminalCheckResponse, TerminalRunResponse};

// ── Faz 2: Kullanıcı Terminali ─────────────────────────────
//
// Kullanıcının kendi yazdığı TEK SATIR komut, agent'larla aynı politika
// motorundan geçer (bkz. agents::user_terminal). İki aşamalı:
//   1) terminal_check_command  → çalıştırmadan karar (allow/ask/deny + argv)
//   2) terminal_run_command    → çalıştır; ask ise `confirmed: true` şart.
// Onay kutusu arayüzdedir ama Rust kendi başına da zorlar.

fn user_terminal_tool(
    rt: &crate::bridge::state::MobileRuntime,
) -> crate::agents::terminal_tool::TerminalAgentTool {
    crate::agents::terminal_tool::TerminalAgentTool::with_workspace(std::path::PathBuf::from(
        &rt.workspace_dir,
    ))
}

/// Komutu çalıştırmadan ayrıştırır ve politikaya sorar.
pub fn terminal_check_command(command_line: String) -> Result<TerminalCheckResponse, String> {
    use crate::types::agent_tool::CallVerdict;

    let rt = require_runtime()?;
    let tool = user_terminal_tool(rt);
    let checked = crate::agents::user_terminal::check(&tool, &command_line)?;

    let (verdict, reason) = match checked.verdict {
        CallVerdict::Allow => ("allow", String::new()),
        CallVerdict::Ask { reason } => ("ask", reason),
        CallVerdict::Deny { reason } => ("deny", reason),
    };
    Ok(TerminalCheckResponse {
        verdict: verdict.to_string(),
        reason,
        argv: checked.argv,
    })
}

/// Komutu çalıştırır. Deny → Err. Ask → `confirmed` true değilse Err.
pub async fn terminal_run_command(
    command_line: String,
    confirmed: bool,
) -> Result<TerminalRunResponse, String> {
    let rt = require_runtime()?;
    let tool = user_terminal_tool(rt);
    let res =
        crate::agents::user_terminal::run(&tool, &rt.audit_log, &command_line, confirmed).await?;
    Ok(TerminalRunResponse {
        success: res.success,
        output: res.output,
        truncated: res.truncated,
    })
}

/// Workspace araçlarını (oku/listele/ara/yaz/…) agent'ın araç listesine ekler.
/// Listede olmak izin vermez: her araç kendi `Workspace*` capability'sini
/// ister ve Governor/RiskEngine zincirinden geçer.
pub(crate) fn append_workspace_tools(
    rt: &crate::bridge::state::MobileRuntime,
    tools: &mut Vec<std::sync::Arc<dyn crate::types::agent_tool::AgentTool>>,
) {
    use crate::agents::workspace_tool::{WorkspaceAgentTool, WorkspaceToolKind};
    let Some(ws) = rt.workspace.as_ref() else {
        return;
    };
    for kind in WorkspaceToolKind::ALL {
        tools.push(std::sync::Arc::new(WorkspaceAgentTool::new(
            ws.clone(),
            kind,
        )));
    }
}
