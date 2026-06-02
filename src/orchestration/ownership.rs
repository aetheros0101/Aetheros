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
pub struct ExecutionOwnership {
    pub execution_id:
        Uuid,

    pub node_id:
        Uuid,

    pub leased_until:
        DateTime<Utc>,
}
