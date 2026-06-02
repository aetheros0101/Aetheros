use chrono::{
    DateTime,
    Utc,
};

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
pub struct AuditEvent {
    pub event_type:
        String,

    pub timestamp:
        DateTime<Utc>,
}
