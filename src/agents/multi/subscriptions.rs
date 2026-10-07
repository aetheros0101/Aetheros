use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSubscription {
    pub event_type: String,
    pub durable: bool,
    #[serde(default)]
    pub filter: Option<String>,
    #[serde(default)]
    pub tenant_id: Option<String>,
}
