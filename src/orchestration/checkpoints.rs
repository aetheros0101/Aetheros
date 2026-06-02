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
pub struct DistributedCheckpoint {
    pub execution_id:
        Uuid,

    pub node_id:
        Uuid,

    pub version:
        u64,

    pub timestamp:
        DateTime<Utc>,
}
