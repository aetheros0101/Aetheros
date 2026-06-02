// ============================================================
// src/remote/node.rs
//
// Sprint 5: RemoteNode'a capabilities eklendi.
// RemoteScheduler capability-aware seçim yapabilsin.
// ============================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeHealth {
    Healthy,
    Degraded,
    Unreachable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeCapability {
    WasmExecution,
    WorkflowExecution,
    AgentExecution,
    AiInference,
    PluginExecution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteNode {
    pub node_id: Uuid,
    pub address: String,
    pub healthy: bool,
    pub last_seen: DateTime<Utc>,
    /// Node'un desteklediği execution türleri.
    /// Scheduler bu listeye göre filtreleme yapar.
    #[serde(default)]
    pub capabilities: Vec<NodeCapability>,
}

impl RemoteNode {
    pub fn new(
        address: impl Into<String>,
        capabilities: Vec<NodeCapability>,
    ) -> Self {
        Self {
            node_id: Uuid::new_v4(),
            address: address.into(),
            healthy: true,
            last_seen: Utc::now(),
            capabilities,
        }
    }

    pub fn touch(&mut self) {
        self.last_seen = Utc::now();
    }

    pub fn mark_unhealthy(&mut self) {
        self.healthy = false;
    }
}
