//! Reasoning / decision trace — host uyumlu çekirdek alanlar.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningPhase {
    #[default]
    Plan,
    StepSelect,
    ToolResult,
    Approval,
    Reflection,
    Error,
}

/// Host runtime `agent_id` / `decision` / `timestamp` literal kullanır.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningTrace {
    pub agent_id: String,
    pub decision: String,
    pub timestamp: DateTime<Utc>,
}

impl ReasoningTrace {
    pub fn new(agent_id: impl Into<String>, _phase: ReasoningPhase, decision: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            decision: decision.into(),
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ReasoningLog {
    pub entries: Vec<ReasoningTrace>,
}

impl ReasoningLog {
    pub fn push(&mut self, entry: ReasoningTrace) {
        self.entries.push(entry);
        if self.entries.len() > 500 {
            self.entries.drain(0..self.entries.len() - 500);
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
