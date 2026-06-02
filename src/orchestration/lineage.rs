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
pub struct ExecutionLineage {
    pub execution_id:
        Uuid,

    pub parent_id:
        Option<Uuid>,

    pub spawned_children:
        Vec<Uuid>,     

    pub root_execution_id:
        Uuid,

    pub correlation_id:
        String,

    pub depth: usize,

    pub created_at:
        DateTime<Utc>,
}
