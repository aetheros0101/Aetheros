use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct AgentSubscription {
    pub event_type:
        String,

    pub durable:
        bool,
}
