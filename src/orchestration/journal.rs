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
pub struct JournalEntry {
    pub sequence_id:
        u64,

    pub execution_id:
        Uuid,

    pub checksum:
        String,

    pub event_type:
        String,

    pub timestamp:
        DateTime<Utc>,
}
