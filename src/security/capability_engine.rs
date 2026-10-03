// ============================================================
// src/security/capability_engine.rs
//
// V10 Sprint 1: Capability Engine
//
// Amaç: AgentCapabilities/AgentCapability struct'ları şu ana kadar
// hiçbir yerde enforce edilmiyordu. Bu modül, bir agent'ın hangi
// capability'lere sahip olduğunu tutan ve her tool çağrısından önce
// "bu agent bunu yapabilir mi?" sorusuna cevap veren tek kontrol
// noktasıdır (RbacGuard'ın capability seviyesindeki karşılığı).
//
// Kullanım deseni RbacGuard ile aynı: stateless karar + ayrı bir
// yerde (burada RwLock<HashMap>) tutulan grant listesi.
// ============================================================

use std::collections::HashMap;
use std::sync::RwLock;

use uuid::Uuid;

use crate::agents::capabilities::{AgentCapabilities, AgentCapability};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityDecision {
    Allowed,
    Denied { reason: String },
}

impl CapabilityDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, CapabilityDecision::Allowed)
    }
}

pub struct CapabilityEngine {
    grants: RwLock<HashMap<Uuid, AgentCapabilities>>,
}

impl CapabilityEngine {
    pub fn new() -> Self {
        Self {
            grants: RwLock::new(HashMap::new()),
        }
    }

    /// Bir agent'a capability seti ata (örn. agent spawn edilirken).
    pub fn grant(&self, agent_id: Uuid, capabilities: AgentCapabilities) {
        self.grants.write().unwrap().insert(agent_id, capabilities);
    }

    /// Agent'ın mevcut grant setine TEK bir capability ekle (varsa
    /// dokunmadan korur, yoksa hepsi false olan bir set açıp üzerine
    /// ekler). `grant()`'ın aksine mevcut grant'ları silmez.
    pub fn grant_capability(&self, agent_id: Uuid, capability: AgentCapability) {
        let mut grants = self.grants.write().unwrap();
        let entry = grants.entry(agent_id).or_insert_with(|| AgentCapabilities {
            workflow_execution: false,
            wasm_execution: false,
            ai_reasoning: false,
            remote_execution: false,
            terminal_execution: false,
        });
        match capability {
            AgentCapability::WorkflowExecution => entry.workflow_execution = true,
            AgentCapability::WasmExecution => entry.wasm_execution = true,
            AgentCapability::AiReasoning => entry.ai_reasoning = true,
            AgentCapability::RemoteExecution => entry.remote_execution = true,
            AgentCapability::TerminalExecution => entry.terminal_execution = true,
        }
    }

    /// Agent'ın tüm capability'lerini iptal et (örn. execution bitince
    /// veya Approval Engine bir ihlal tespit ettiğinde).
    pub fn revoke(&self, agent_id: &Uuid) {
        self.grants.write().unwrap().remove(agent_id);
    }

    /// Tek kontrol noktası: bu agent, `required` capability'sini
    /// gerektiren bir tool'u çağırabilir mi?
    ///
    /// `required = None` → tool herhangi bir capability talep etmiyor,
    /// otomatik izin verilir (geriye dönük uyumluluk: mevcut tool'lar
    /// `required_capability()`'yi implement etmek zorunda değil).
    pub fn check(&self, agent_id: &Uuid, required: Option<AgentCapability>) -> CapabilityDecision {
        let Some(required) = required else {
            return CapabilityDecision::Allowed;
        };

        let grants = self.grants.read().unwrap();
        let Some(caps) = grants.get(agent_id) else {
            return CapabilityDecision::Denied {
                reason: format!(
                    "agent {agent_id} için hiç capability grant kaydı yok"
                ),
            };
        };

        let has = match required {
            AgentCapability::WorkflowExecution => caps.workflow_execution,
            AgentCapability::WasmExecution => caps.wasm_execution,
            AgentCapability::AiReasoning => caps.ai_reasoning,
            AgentCapability::RemoteExecution => caps.remote_execution,
            AgentCapability::TerminalExecution => caps.terminal_execution,
        };

        if has {
            CapabilityDecision::Allowed
        } else {
            CapabilityDecision::Denied {
                reason: format!(
                    "agent {agent_id}, {required:?} capability'sine sahip değil"
                ),
            }
        }
    }
}

impl Default for CapabilityEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_requirement_is_always_allowed() {
        let engine = CapabilityEngine::new();
        let agent_id = Uuid::new_v4();
        assert!(engine.check(&agent_id, None).is_allowed());
    }

    #[test]
    fn unknown_agent_is_denied_when_capability_required() {
        let engine = CapabilityEngine::new();
        let agent_id = Uuid::new_v4();
        let decision = engine.check(&agent_id, Some(AgentCapability::WasmExecution));
        assert!(!decision.is_allowed());
    }

    #[test]
    fn granted_capability_is_allowed() {
        let engine = CapabilityEngine::new();
        let agent_id = Uuid::new_v4();
        engine.grant(
            agent_id,
            AgentCapabilities {
                workflow_execution: false,
                wasm_execution: true,
                ai_reasoning: false,
                remote_execution: false,
                terminal_execution: false,
            },
        );
        assert!(engine
            .check(&agent_id, Some(AgentCapability::WasmExecution))
            .is_allowed());
        assert!(!engine
            .check(&agent_id, Some(AgentCapability::RemoteExecution))
            .is_allowed());
    }

    #[test]
    fn incremental_grant_does_not_clobber_existing_grants() {
        let engine = CapabilityEngine::new();
        let agent_id = Uuid::new_v4();
        engine.grant_capability(agent_id, AgentCapability::WasmExecution);
        engine.grant_capability(agent_id, AgentCapability::AiReasoning);

        assert!(engine
            .check(&agent_id, Some(AgentCapability::WasmExecution))
            .is_allowed());
        assert!(engine
            .check(&agent_id, Some(AgentCapability::AiReasoning))
            .is_allowed());
        assert!(!engine
            .check(&agent_id, Some(AgentCapability::RemoteExecution))
            .is_allowed());
    }
}
