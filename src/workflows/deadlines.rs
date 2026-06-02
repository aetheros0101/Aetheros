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
pub struct WorkflowDeadline {
    pub deadline:
        DateTime<Utc>,

    pub hard_timeout:
        bool,
}
