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
pub struct TraceSpan {
    pub trace_id:
        String,

    pub span_id:
        String,

    pub started_at:
        DateTime<Utc>,
}
