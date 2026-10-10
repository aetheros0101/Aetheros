// ============================================================
// bridge/api/agent_api.rs
//
// Agent başlatma, durum ve listeleme.
// (agent.rs is already used for registries in bridge/)
// ============================================================

use tracing::info;

use crate::bridge::api::error::{describe_runtime_error, require_runtime};
use crate::bridge::api::helpers::parse_capability;
use crate::bridge::api::terminal::append_workspace_tools;
use crate::bridge::types::{AgentStartResponse, AgentStatusResponse};

// ── Agent fonksiyonları (FRB) ─────────────────────────────────

/// Agent başlat → execution_id döner.
///
/// `capabilities`: agent'a BAŞLANGIÇTA verilecek yetkiler
/// ("wasm_execution" | "workflow_execution" | "ai_reasoning" |
/// "remote_execution" | "terminal_execution"). Boş liste = hiç yetki
/// (capability gerektiren hiçbir tool çalışmaz). Yetkiler agent
/// spawn edilmeden ÖNCE verilir — `grant_agent_capability` ile sonradan
/// vermenin yarışı yoktur. Bilinmeyen bir ad varsa HİÇBİR şey
/// başlatılmaz/verilmez (kısmi durum bırakmaz).
pub async fn start_agent(
    objective: String,
    max_steps: usize,
    max_tokens: usize,
    capabilities: Vec<String>,
) -> Result<AgentStartResponse, String> {
    let rt = require_runtime()?;

    // Önce HEPSİNİ doğrula (kısmi grant yok).
    let mut caps = Vec::new();
    for name in &capabilities {
        let cap = parse_capability(name)?;
        if !caps.contains(&cap) {
            caps.push(cap);
        }
    }

    let execution_id = uuid::Uuid::new_v4();
    let agent_id = uuid::Uuid::new_v4();

    // Yetkiler, agent'ın execution'ı spawn edilmeden ÖNCE (B11).
    for cap in &caps {
        rt.capability_engine.grant_capability(agent_id, *cap);
    }
    if !caps.is_empty() {
        info!(agent_id = %agent_id, capabilities = ?capabilities, "Agent başlangıç yetkileri verildi");
    }
    let started_at = chrono::Utc::now();
    let objective_c = objective.clone();

    rt.agent_registry.insert(
        execution_id,
        crate::bridge::agent::AgentEntry {
            execution_id,
            agent_id,
            objective: objective.clone(),
            status: "running".into(),
            error: None,
            started_at,
            finished_at: None,
            pending_approval_id: None,
        },
    );

    let registry = rt.agent_registry.clone();
    let context = crate::agents::context::AgentContext {
        agent_id,
        execution_id,
        workflow_id: None,
    };
    let budget = crate::agents::budget::AgentExecutionBudget {
        max_tokens,
        max_steps,
        max_runtime_seconds: 300,
    };
    let ai_router = rt.ai_router.clone();
    let capability_engine = rt.capability_engine.clone();
    let risk_engine = rt.risk_engine.clone();
    let approval_store = rt.approval_store.clone();
    let audit_log = rt.audit_log.clone();

    // V10 Sprint 1b: agent'ın araç kutusu artık boş değil — registry'de
    // kayıtlı her script bir ScriptTool olarak agent'a sunuluyor.
    // Hangisinin GERÇEKTEN çalışabileceğine CapabilityEngine karar verir
    // (bkz. grant_agent_capability) — burada listelenmek izin vermez.
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
    // V10 Faz 1: terminal her agent'a sunuluyor — listede olmak izin
    // vermez, TerminalExecution capability + Governor onayı hâlâ şart.
    tools.push(std::sync::Arc::new(
        crate::agents::terminal_tool::TerminalAgentTool::with_workspace(std::path::PathBuf::from(
            &rt.workspace_dir,
        )),
    ));
    append_workspace_tools(rt, &mut tools);

    tokio::spawn(async move {
        let result = crate::agents::executor::AgentExecutor::execute(
            context,
            objective_c,
            budget,
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
                    // V10 Sprint 5: execution bitmedi, DURAKLADI — finished_at
                    // BİLEREK set edilmiyor, respond_to_approval() devam
                    // ettirene ya da kalıcı reddedene kadar "bitmemiş" sayılır.
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

    Ok(AgentStartResponse {
        execution_id: execution_id.to_string(),
        agent_id: agent_id.to_string(),
        status: "running".into(),
    })
}

/// Agent execution durumunu sorgula.
pub fn get_agent_status(execution_id: String) -> Result<AgentStatusResponse, String> {
    let rt = require_runtime()?;

    let uuid = uuid::Uuid::parse_str(&execution_id)
        .map_err(|_| format!("Geçersiz execution_id: {execution_id}"))?;

    rt.agent_registry
        .get(&uuid)
        .map(|e| AgentStatusResponse {
            execution_id: e.execution_id.to_string(),
            agent_id: e.agent_id.to_string(),
            objective: e.objective.clone(),
            status: e.status.clone(),
            error: e.error.clone(),
            started_at: e.started_at.timestamp_millis(),
            finished_at: e.finished_at.map(|t| t.timestamp_millis()),
            pending_approval_id: e.pending_approval_id.map(|id| id.to_string()),
        })
        .ok_or_else(|| format!("Agent bulunamadı: {execution_id}"))
}

/// Tüm agent execution'larını listele.
pub fn list_agents(limit: usize) -> Result<Vec<AgentStatusResponse>, String> {
    let rt = require_runtime()?;

    let mut list: Vec<AgentStatusResponse> = rt
        .agent_registry
        .iter()
        .take(limit)
        .map(|e| AgentStatusResponse {
            execution_id: e.execution_id.to_string(),
            agent_id: e.agent_id.to_string(),
            objective: e.objective.clone(),
            status: e.status.clone(),
            error: e.error.clone(),
            started_at: e.started_at.timestamp_millis(),
            finished_at: e.finished_at.map(|t| t.timestamp_millis()),
            pending_approval_id: e.pending_approval_id.map(|id| id.to_string()),
        })
        .collect();

    // En yeni önce
    list.sort_by_key(|e| std::cmp::Reverse(e.started_at));
    Ok(list)
}
