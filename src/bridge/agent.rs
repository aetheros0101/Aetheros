// ============================================================
// src/bridge/agent.rs
//
// FRB bridge için in-memory agent/workflow execution kaydı.
// REST tarafındaki AppState'te benzer yapı mevcut —
// mobile bridge'e özgü ayrı registry buraya taşındı.
// ============================================================

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

// ── Agent ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AgentEntry {
    pub execution_id: Uuid,
    pub agent_id: Uuid,
    pub objective: String,
    pub status: String, // "running" | "completed" | "failed" | "pending_approval"
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    /// V10 Sprint 5: status "pending_approval" olduğunda dolu —
    /// respond_to_approval()'a geçilecek kimlik.
    pub pending_approval_id: Option<Uuid>,
}

pub type AgentRegistry = Arc<DashMap<Uuid, AgentEntry>>;

// ── Workflow ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WorkflowEntry {
    pub workflow_id: Uuid,
    pub name: String,
    pub status: String, // "running" | "completed" | "failed"
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

pub type WorkflowRegistry = Arc<DashMap<Uuid, WorkflowEntry>>;
