use serde::{
    Deserialize,
    Serialize,
};

use uuid::Uuid;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct AgentRegistration {
    pub agent_id: Uuid,

    pub name: String,
}
