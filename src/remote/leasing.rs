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
pub struct TaskLease {
    pub lease_id: Uuid,

    pub execution_id:
        Uuid,

    pub node_id: Uuid,

    pub expires_at:
        DateTime<Utc>,

    pub workflow_id:
        Option<Uuid>,
}
