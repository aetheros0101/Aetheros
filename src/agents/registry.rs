use std::collections::HashSet;

use uuid::Uuid;

use crate::agents::capabilities::AgentCapability;

pub struct RegisteredAgent {
    pub id: Uuid,

    pub name: String,

    pub capabilities:
        HashSet<
            AgentCapability,
        >,
}

pub struct AgentRegistry {
    agents:
        Vec<RegisteredAgent>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents:
                Vec::new(),
        }
    }

    pub fn register(
        &mut self,
        agent:
            RegisteredAgent,
    ) {
        self.agents.push(
            agent,
        );
    }

    pub fn agents(
        &self,
    ) -> &[RegisteredAgent] {
        &self.agents
    }
}
