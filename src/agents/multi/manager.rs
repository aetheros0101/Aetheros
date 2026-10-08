//! Multi-agent yöneticisi: kayıt, kota, isolation, lease.

use crate::agents::capacity::{AgentCapacity, QuotaPolicy, QuotaUsage};
use crate::agents::errors::AgentError;
use crate::agents::registry::AgentRegistry;
use crate::agents::state::AgentState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Isolation kapsamı — agent'lar birbirinin workspace/tenant'ına sızmasın.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct IsolationScope {
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    /// Workspace kökünün mutlak yolu (path guard ile hizalı).
    #[serde(default)]
    pub workspace_root: Option<String>,
}

impl IsolationScope {
    pub fn matches(&self, other: &IsolationScope) -> bool {
        fn eq_opt(a: &Option<String>, b: &Option<String>) -> bool {
            match (a, b) {
                (None, None) => true,
                (Some(x), Some(y)) => x == y,
                _ => false,
            }
        }
        eq_opt(&self.tenant_id, &other.tenant_id)
            && eq_opt(&self.project_id, &other.project_id)
            && eq_opt(&self.workspace_root, &other.workspace_root)
    }

    pub fn tenant(&self) -> Option<&str> {
        self.tenant_id.as_deref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDescriptor {
    pub id: Uuid,
    pub name: String,
    pub state: AgentState,
    #[serde(default)]
    pub policy_profile: String,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub isolation: IsolationScope,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    /// Aktif execution varsa.
    #[serde(default)]
    pub current_execution_id: Option<Uuid>,
}

/// Çalışan execution lease'i.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLease {
    pub lease_id: Uuid,
    pub agent_id: Uuid,
    pub execution_id: Uuid,
    pub isolation: IsolationScope,
    pub acquired_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerStats {
    pub registered: usize,
    pub active_agents: usize,
    pub active_leases: usize,
    pub global_capacity: AgentCapacity,
    pub quota: QuotaUsage,
}

pub struct AgentManager {
    registry: AgentRegistry,
    capacity: AgentCapacity,
    quota_policy: QuotaPolicy,
    quota_usage: QuotaUsage,
    leases: Vec<ExecutionLease>,
}

impl AgentManager {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            registry: AgentRegistry::new(),
            capacity: AgentCapacity {
                max_concurrent_tasks: max_concurrent,
                active_tasks: 0,
            },
            quota_policy: QuotaPolicy {
                max_concurrent,
                ..QuotaPolicy::default()
            },
            quota_usage: QuotaUsage::default(),
            leases: Vec::new(),
        }
    }

    pub fn with_quota(mut self, policy: QuotaPolicy) -> Self {
        self.capacity.max_concurrent_tasks = policy.max_concurrent;
        self.quota_policy = policy;
        self
    }

    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    pub fn capacity(&self) -> &AgentCapacity {
        &self.capacity
    }

    pub fn quota_policy(&self) -> &QuotaPolicy {
        &self.quota_policy
    }

    pub fn stats(&self) -> ManagerStats {
        ManagerStats {
            registered: self.registry.len(),
            active_agents: self.registry.active().len(),
            active_leases: self.leases.len(),
            global_capacity: self.capacity.clone(),
            quota: self.quota_usage.clone(),
        }
    }

    pub fn register(
        &mut self,
        name: impl Into<String>,
        profile: impl Into<String>,
    ) -> AgentDescriptor {
        self.register_isolated(name, profile, IsolationScope::default())
    }

    pub fn register_isolated(
        &mut self,
        name: impl Into<String>,
        profile: impl Into<String>,
        isolation: IsolationScope,
    ) -> AgentDescriptor {
        let id = Uuid::new_v4();
        let desc = AgentDescriptor {
            id,
            name: name.into(),
            state: AgentState::Registered,
            policy_profile: profile.into(),
            labels: Vec::new(),
            tenant_id: isolation.tenant_id.clone(),
            isolation,
            created_at: Some(Utc::now()),
            current_execution_id: None,
        };
        self.registry.insert(desc.clone());
        desc
    }

    /// Kota + capacity slot al; isolation lease üret.
    pub fn try_acquire_execution(
        &mut self,
        agent_id: Uuid,
        execution_id: Uuid,
    ) -> Result<ExecutionLease, AgentError> {
        let desc = self
            .registry
            .get(agent_id)
            .ok_or_else(|| AgentError::validation(format!("agent {agent_id} not found")))?
            .clone();

        self.quota_usage
            .can_acquire(&self.quota_policy, desc.isolation.tenant())
            .map_err(AgentError::budget)?;

        if self.capacity.is_full() {
            return Err(AgentError::budget(format!(
                "max concurrent agents {} reached",
                self.capacity.max_concurrent_tasks
            )));
        }

        // Bir agent aynı anda tek lease tutar. Önceden aynı execution_id ile
        // tekrar acquire sayaçları iki kez artırıp lease'i çiftliyordu.
        if self.leases.iter().any(|l| l.agent_id == agent_id) {
            return Err(AgentError::validation(format!(
                "agent {agent_id} already has an active execution lease"
            )));
        }

        self.quota_usage.acquire(desc.isolation.tenant());
        self.capacity.active_tasks += 1;

        let lease = ExecutionLease {
            lease_id: Uuid::new_v4(),
            agent_id,
            execution_id,
            isolation: desc.isolation.clone(),
            acquired_at: Utc::now(),
        };
        self.leases.push(lease.clone());

        if let Some(a) = self.registry.get_mut(agent_id) {
            a.current_execution_id = Some(execution_id);
            a.state = AgentState::Executing;
        }

        Ok(lease)
    }

    /// Geriye uyum: sadece capacity slot.
    pub fn try_acquire_slot(&mut self) -> Result<(), AgentError> {
        if self.capacity.is_full() {
            return Err(AgentError::budget(format!(
                "max concurrent agents {} reached",
                self.capacity.max_concurrent_tasks
            )));
        }
        self.capacity.active_tasks += 1;
        self.quota_usage.acquire(None);
        Ok(())
    }

    pub fn release_slot(&mut self) {
        self.capacity.active_tasks = self.capacity.active_tasks.saturating_sub(1);
        self.quota_usage.release(None);
    }

    pub fn release_lease(&mut self, lease_id: Uuid) -> Result<(), AgentError> {
        let idx = self
            .leases
            .iter()
            .position(|l| l.lease_id == lease_id)
            .ok_or_else(|| AgentError::validation(format!("lease {lease_id} not found")))?;
        let lease = self.leases.remove(idx);
        self.capacity.active_tasks = self.capacity.active_tasks.saturating_sub(1);
        self.quota_usage.release(lease.isolation.tenant());

        if let Some(a) = self.registry.get_mut(lease.agent_id) {
            if a.current_execution_id == Some(lease.execution_id) {
                a.current_execution_id = None;
            }
            if a.state.is_active() {
                a.state = AgentState::Completed;
            }
        }
        Ok(())
    }

    pub fn release_execution(&mut self, agent_id: Uuid, execution_id: Uuid) {
        if let Some(idx) = self
            .leases
            .iter()
            .position(|l| l.agent_id == agent_id && l.execution_id == execution_id)
        {
            let lease = self.leases.remove(idx);
            self.capacity.active_tasks = self.capacity.active_tasks.saturating_sub(1);
            self.quota_usage.release(lease.isolation.tenant());
        }
        if let Some(a) = self.registry.get_mut(agent_id)
            && a.current_execution_id == Some(execution_id)
        {
            a.current_execution_id = None;
        }
    }

    /// Isolation ihlali: lease'in scope'u beklenen ile uyuşuyor mu?
    pub fn assert_isolation(
        &self,
        lease_id: Uuid,
        expected: &IsolationScope,
    ) -> Result<(), AgentError> {
        let lease = self
            .leases
            .iter()
            .find(|l| l.lease_id == lease_id)
            .ok_or_else(|| AgentError::validation(format!("lease {lease_id} not found")))?;
        if !lease.isolation.matches(expected) {
            return Err(AgentError::capability(format!(
                "isolation mismatch for lease {lease_id}"
            )));
        }
        Ok(())
    }

    pub fn set_state(&mut self, id: Uuid, state: AgentState) -> Result<(), AgentError> {
        self.registry.set_state(id, state)
    }

    pub fn unregister(&mut self, id: Uuid) -> Result<AgentDescriptor, AgentError> {
        if self.leases.iter().any(|l| l.agent_id == id) {
            return Err(AgentError::validation(
                "cannot unregister agent with active lease",
            ));
        }
        self.registry
            .remove(id)
            .ok_or_else(|| AgentError::validation(format!("agent {id} not found")))
    }

    pub fn list(&self) -> Vec<AgentDescriptor> {
        self.registry.list()
    }

    pub fn list_leases(&self) -> &[ExecutionLease] {
        &self.leases
    }
}

pub type SharedAgentManager = Arc<std::sync::Mutex<AgentManager>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::errors::AgentErrorKind;

    fn scope(tenant: &str) -> IsolationScope {
        IsolationScope {
            tenant_id: Some(tenant.to_string()),
            ..IsolationScope::default()
        }
    }

    #[test]
    fn register_and_list() {
        let mut m = AgentManager::new(2);
        let d = m.register("a", "default");
        assert_eq!(d.state, AgentState::Registered);
        assert_eq!(m.list().len(), 1);
        assert_eq!(m.stats().registered, 1);
        assert_eq!(m.stats().active_agents, 0);
    }

    #[test]
    fn acquire_and_release_lease_roundtrip() {
        let mut m = AgentManager::new(2);
        let d = m.register("a", "default");
        let exec = Uuid::new_v4();

        let lease = m.try_acquire_execution(d.id, exec).expect("acquire");
        assert_eq!(m.capacity().active_tasks, 1);
        assert_eq!(m.stats().active_leases, 1);
        let got = m.registry().get(d.id).unwrap();
        assert_eq!(got.state, AgentState::Executing);
        assert_eq!(got.current_execution_id, Some(exec));

        m.release_lease(lease.lease_id).expect("release");
        assert_eq!(m.capacity().active_tasks, 0);
        assert_eq!(m.stats().quota.global_active, 0);
        let got = m.registry().get(d.id).unwrap();
        assert_eq!(got.state, AgentState::Completed);
        assert_eq!(got.current_execution_id, None);
    }

    #[test]
    fn acquire_unknown_agent_is_validation_error() {
        let mut m = AgentManager::new(1);
        let err = m
            .try_acquire_execution(Uuid::new_v4(), Uuid::new_v4())
            .unwrap_err();
        assert_eq!(err.kind, AgentErrorKind::Validation);
    }

    #[test]
    fn global_capacity_is_enforced() {
        let mut m = AgentManager::new(1);
        let a = m.register("a", "p");
        let b = m.register("b", "p");
        m.try_acquire_execution(a.id, Uuid::new_v4()).unwrap();
        let err = m.try_acquire_execution(b.id, Uuid::new_v4()).unwrap_err();
        assert_eq!(err.kind, AgentErrorKind::Budget);
        assert_eq!(m.capacity().active_tasks, 1);
    }

    #[test]
    fn per_tenant_quota_is_enforced_and_isolated() {
        let policy = QuotaPolicy {
            max_concurrent: 10,
            max_per_tenant: Some(1),
            ..QuotaPolicy::default()
        };
        let mut m = AgentManager::new(10).with_quota(policy);
        let a1 = m.register_isolated("a1", "p", scope("t1"));
        let a2 = m.register_isolated("a2", "p", scope("t1"));
        let b1 = m.register_isolated("b1", "p", scope("t2"));

        let l1 = m.try_acquire_execution(a1.id, Uuid::new_v4()).unwrap();
        let err = m.try_acquire_execution(a2.id, Uuid::new_v4()).unwrap_err();
        assert_eq!(err.kind, AgentErrorKind::Budget);
        // Başka tenant etkilenmez.
        m.try_acquire_execution(b1.id, Uuid::new_v4()).unwrap();
        // Slot boşalınca t1 tekrar alabilir.
        m.release_lease(l1.lease_id).unwrap();
        m.try_acquire_execution(a2.id, Uuid::new_v4()).unwrap();
    }

    #[test]
    fn agent_cannot_hold_two_leases() {
        // Regresyon: aynı execution_id ile ikinci acquire sayaçları çiftliyordu.
        let mut m = AgentManager::new(4);
        let d = m.register("a", "p");
        let exec = Uuid::new_v4();
        m.try_acquire_execution(d.id, exec).unwrap();

        let same = m.try_acquire_execution(d.id, exec).unwrap_err();
        assert_eq!(same.kind, AgentErrorKind::Validation);
        let other = m.try_acquire_execution(d.id, Uuid::new_v4()).unwrap_err();
        assert_eq!(other.kind, AgentErrorKind::Validation);

        assert_eq!(m.capacity().active_tasks, 1);
        assert_eq!(m.list_leases().len(), 1);
    }

    #[test]
    fn release_unknown_lease_errors() {
        let mut m = AgentManager::new(1);
        let err = m.release_lease(Uuid::new_v4()).unwrap_err();
        assert_eq!(err.kind, AgentErrorKind::Validation);
    }

    #[test]
    fn release_execution_frees_slot_and_is_idempotent() {
        let mut m = AgentManager::new(1);
        let d = m.register("a", "p");
        let exec = Uuid::new_v4();
        m.try_acquire_execution(d.id, exec).unwrap();

        m.release_execution(d.id, exec);
        m.release_execution(d.id, exec); // ikinci çağrı zarar vermemeli
        assert_eq!(m.capacity().active_tasks, 0);
        assert!(m.list_leases().is_empty());
        assert_eq!(m.registry().get(d.id).unwrap().current_execution_id, None);
    }

    #[test]
    fn slot_api_never_underflows() {
        let mut m = AgentManager::new(1);
        m.try_acquire_slot().unwrap();
        assert!(m.try_acquire_slot().is_err());
        m.release_slot();
        m.release_slot();
        assert_eq!(m.capacity().active_tasks, 0);
        assert_eq!(m.stats().quota.global_active, 0);
    }

    #[test]
    fn isolation_assertion_detects_mismatch() {
        let mut m = AgentManager::new(2);
        let d = m.register_isolated("a", "p", scope("t1"));
        let lease = m.try_acquire_execution(d.id, Uuid::new_v4()).unwrap();

        assert!(m.assert_isolation(lease.lease_id, &scope("t1")).is_ok());
        let err = m
            .assert_isolation(lease.lease_id, &scope("t2"))
            .unwrap_err();
        assert_eq!(err.kind, AgentErrorKind::Capability);
        let missing = m
            .assert_isolation(Uuid::new_v4(), &scope("t1"))
            .unwrap_err();
        assert_eq!(missing.kind, AgentErrorKind::Validation);
    }

    #[test]
    fn isolation_scope_matches_requires_all_fields_equal() {
        let a = IsolationScope {
            tenant_id: Some("t".into()),
            project_id: Some("p".into()),
            workspace_root: None,
        };
        assert!(a.matches(&a.clone()));
        let mut b = a.clone();
        b.workspace_root = Some("/w".into());
        assert!(!a.matches(&b));
        assert!(!IsolationScope::default().matches(&a));
        assert!(IsolationScope::default().matches(&IsolationScope::default()));
    }

    #[test]
    fn unregister_blocked_while_leased() {
        let mut m = AgentManager::new(1);
        let d = m.register("a", "p");
        let lease = m.try_acquire_execution(d.id, Uuid::new_v4()).unwrap();
        assert!(m.unregister(d.id).is_err());

        m.release_lease(lease.lease_id).unwrap();
        assert_eq!(m.unregister(d.id).unwrap().id, d.id);
        assert!(m.unregister(d.id).is_err()); // artık yok
    }

    #[test]
    fn set_state_unknown_agent_errors() {
        let mut m = AgentManager::new(1);
        assert!(m.set_state(Uuid::new_v4(), AgentState::Failed).is_err());
        let d = m.register("a", "p");
        m.set_state(d.id, AgentState::Failed).unwrap();
        assert_eq!(m.registry().get(d.id).unwrap().state, AgentState::Failed);
    }

    #[test]
    fn registry_filters_by_tenant() {
        let mut m = AgentManager::new(4);
        m.register_isolated("a", "p", scope("t1"));
        m.register_isolated("b", "p", scope("t2"));
        m.register("c", "p");
        assert_eq!(m.registry().list_by_tenant("t1").len(), 1);
        assert_eq!(m.registry().list_by_tenant("nope").len(), 0);
    }
}
