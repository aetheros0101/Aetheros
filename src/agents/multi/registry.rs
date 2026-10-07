use crate::agents::errors::AgentError;
use crate::agents::manager::AgentDescriptor;
use crate::agents::state::AgentState;
use std::collections::HashMap;
use uuid::Uuid;

pub struct AgentRegistry {
    agents: HashMap<Uuid, AgentDescriptor>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    pub fn insert(&mut self, desc: AgentDescriptor) {
        self.agents.insert(desc.id, desc);
    }

    pub fn get(&self, id: Uuid) -> Option<&AgentDescriptor> {
        self.agents.get(&id)
    }

    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut AgentDescriptor> {
        self.agents.get_mut(&id)
    }

    pub fn set_state(&mut self, id: Uuid, state: AgentState) -> Result<(), AgentError> {
        let a = self
            .agents
            .get_mut(&id)
            .ok_or_else(|| AgentError::validation(format!("agent {id} not found")))?;
        a.state = state;
        Ok(())
    }

    pub fn remove(&mut self, id: Uuid) -> Option<AgentDescriptor> {
        self.agents.remove(&id)
    }

    pub fn list(&self) -> Vec<AgentDescriptor> {
        self.agents.values().cloned().collect()
    }

    pub fn list_by_tenant(&self, tenant: &str) -> Vec<AgentDescriptor> {
        self.agents
            .values()
            .filter(|d| d.tenant_id.as_deref() == Some(tenant))
            .cloned()
            .collect()
    }

    pub fn active(&self) -> Vec<AgentDescriptor> {
        self.agents
            .values()
            .filter(|d| d.state.is_active())
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.agents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
