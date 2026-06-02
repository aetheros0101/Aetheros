use uuid::Uuid;

use crate::agents::registry::{
    AgentRegistry,
    RegisteredAgent,
};

pub struct AgentManager {
    registry:
        AgentRegistry,
}

impl AgentManager {
    pub fn new() -> Self {
        Self {
            registry:
                AgentRegistry::new(),
        }
    }

    pub fn register(
        &mut self,
        agent:
            RegisteredAgent,
    ) {
        self.registry
            .register(agent);
    }

    pub fn total_agents(
        &self,
    ) -> usize {
        self.registry
            .agents()
            .len()
    }

    pub fn agent_ids(
        &self,
    ) -> Vec<Uuid> {
        self.registry
            .agents()
            .iter()
            .map(|agent| {
                agent.id
            })
            .collect()
    }
}
