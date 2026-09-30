// ============================================================
// src/agents/approval.rs
//
// V10 Sprint 5: Approval Engine — durdur & sonra devam ettir.
//
// SecurityGovernor bir ToolCall için RequiresApproval dediğinde,
// AgentRuntime execution'ı tool'u hiç invoke etmeden DURDURUR ve
// devam edebilmesi için gereken her şeyi burada saklar:
//   - hangi ToolCall onay bekliyor (ve neden)
//   - execution devam ederse çalıştırılacak KALAN adımlar
//   - kalan budget
//
// KAPSAM NOTU: Bu sprint'te depo in-memory (Arc<RwLock<HashMap>>) —
// ScriptRegistry/CapabilityEngine ile aynı desen, uygulama/runtime
// süreci ayakta olduğu sürece hayatta kalır. Süreç yeniden başlarsa
// bekleyen onaylar kaybolur. Disk'e (PersistenceEngine üzerinden)
// yazıp süreç yeniden başlasa bile hayatta kalmasını sağlamak,
// bilerek ayrı bir sertleştirme adımı olarak bırakıldı.
// ============================================================

use std::collections::HashMap;
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::plans::{AgentPlanStep, ToolCall};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApproval {
    pub id: Uuid,
    pub context: AgentContext,
    pub objective: String,
    /// Onay bekleyen, henüz invoke EDİLMEMİŞ çağrı.
    pub tool_call: ToolCall,
    /// SecurityGovernor'ın RequiresApproval derken verdiği gerekçe.
    pub reason: String,
    /// Onaylanırsa (bu tool_call çalıştırıldıktan SONRA) devam
    /// edilecek adımlar — mevcut adım hariç.
    pub remaining_steps: Vec<AgentPlanStep>,
    /// Duraklatma anındaki kalan budget (harcanan düşülmüş).
    pub budget: AgentExecutionBudget,
    pub created_at: DateTime<Utc>,
}

pub struct ApprovalStore {
    pending: RwLock<HashMap<Uuid, PendingApproval>>,
}

impl ApprovalStore {
    pub fn new() -> Self {
        Self {
            pending: RwLock::new(HashMap::new()),
        }
    }

    pub fn add(&self, approval: PendingApproval) {
        self.pending.write().unwrap().insert(approval.id, approval);
    }

    pub fn list(&self) -> Vec<PendingApproval> {
        self.pending.read().unwrap().values().cloned().collect()
    }

    pub fn get(&self, id: &Uuid) -> Option<PendingApproval> {
        self.pending.read().unwrap().get(id).cloned()
    }

    /// Kaydı çıkar ve döndür — onaylanınca ya da reddedilince
    /// çağrılır; iki kere işlenmesin diye kaldırma atomik.
    pub fn take(&self, id: &Uuid) -> Option<PendingApproval> {
        self.pending.write().unwrap().remove(id)
    }
}

impl Default for ApprovalStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: Uuid) -> PendingApproval {
        PendingApproval {
            id,
            context: AgentContext {
                agent_id: Uuid::new_v4(),
                execution_id: Uuid::new_v4(),
                workflow_id: None,
            },
            objective: "test".to_string(),
            tool_call: ToolCall {
                tool_name: "risky_tool".to_string(),
                arguments: vec![],
            },
            reason: "high risk".to_string(),
            remaining_steps: vec![],
            budget: AgentExecutionBudget {
                max_tokens: 100,
                max_steps: 5,
                max_runtime_seconds: 30,
            },
            created_at: Utc::now(),
        }
    }

    #[test]
    fn add_and_list_roundtrip() {
        let store = ApprovalStore::new();
        let id = Uuid::new_v4();
        store.add(sample(id));
        assert_eq!(store.list().len(), 1);
        assert!(store.get(&id).is_some());
    }

    #[test]
    fn take_removes_and_returns_once() {
        let store = ApprovalStore::new();
        let id = Uuid::new_v4();
        store.add(sample(id));

        assert!(store.take(&id).is_some());
        assert!(store.take(&id).is_none(), "ikinci take None dönmeli");
        assert!(store.list().is_empty());
    }
}
