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
pub struct DistributedTrace {
    pub node_id: String,

    pub execution_id:
        String,

    pub leader:
        bool,

    pub timestamp:
        DateTime<Utc>,
}
