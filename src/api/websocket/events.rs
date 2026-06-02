use serde::{
    Deserialize,
    Serialize,
};

use crate::orchestration::events::OrchestrationEvent;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct WebsocketEvent {
    pub stream: String,

    pub payload:
        OrchestrationEvent,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct RealtimeEvent {
    pub event_type:
        String,

    pub payload:
        String,
}
