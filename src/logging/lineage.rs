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
pub struct LineageEvent {
    pub execution_id:
        Uuid,

    pub correlation_id:
        String,

    pub parent_execution_id:
        Option<Uuid>,

    pub timestamp:
        DateTime<Utc>,
}
