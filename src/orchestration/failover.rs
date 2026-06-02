use uuid::Uuid;

pub struct FailoverDecision {
    pub execution_id:
        Uuid,

    pub from_node:
        Uuid,

    pub to_node:
        Uuid,
}

pub struct FailoverEngine;

impl FailoverEngine {
    pub fn should_failover(
        healthy: bool,
    ) -> bool {
        !healthy
    }
}
