use chrono::{
    DateTime,
    Utc,
};

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
pub struct ExecutionSnapshot {
    pub execution_id:
        Uuid,

    pub snapshot_version:
        u64,

    pub event_offset:
        u64,

    pub created_at:
        DateTime<Utc>,
}
