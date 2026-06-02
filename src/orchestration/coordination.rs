// ============================================================
// src/orchestration/coordination.rs
//
// Faz 5 Düzeltmesi: Döngüsel Bağımlılık Kırıldı
//
// ÖNCE:
//   use crate::workflows::execution_graph::WorkflowExecutionGraph;
//   orchestration → workflows bağımlılığı.
//   workflows → orchestration bağımlılığı da vardı.
//   → Döngüsel çapraz bağımlılık.
//
// SONRA:
//   use crate::orchestration::graph::ExecutionGraph;
//   Her iki modül de orchestration::graph'a bağlı.
//   workflows → orchestration (tek yön) ✓
// ============================================================

use chrono::{
    DateTime,
    Utc,
};

use serde::{
    Deserialize,
    Serialize,
};

use uuid::Uuid;

// [DÜZELTME] workflows bağımlılığı kaldırıldı.
// orchestration kendi graph tipini kullanır.
use crate::orchestration::graph::ExecutionGraph;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct CoordinationLease {
    pub coordination_id: Uuid,
    pub owner_node: Uuid,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl CoordinationLease {
    pub fn expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

pub struct ExecutionCoordinator {
    completed: std::collections::HashSet<Uuid>,
}

impl ExecutionCoordinator {
    pub fn new() -> Self {
        Self {
            completed: std::collections::HashSet::new(),
        }
    }

    /// Bağımlılıkları karşılanmış, çalışmaya hazır node'ları döndür.
    pub fn ready_nodes(
        &self,
        graph: &ExecutionGraph,
    ) -> Vec<Uuid> {
        graph
            .nodes
            .iter()
            .filter(|node| {
                !self.completed.contains(&node.id)
                    && node
                        .dependencies
                        .iter()
                        .all(|dep| self.completed.contains(dep))
            })
            .map(|node| node.id)
            .collect()
    }

    pub fn mark_completed(&mut self, id: Uuid) {
        self.completed.insert(id);
    }

    pub fn is_complete(
        &self,
        graph: &ExecutionGraph,
    ) -> bool {
        graph
            .nodes
            .iter()
            .all(|node| self.completed.contains(&node.id))
    }
}
