use serde::{
    Deserialize,
    Serialize,
};

use crate::types::timestamps::Timestamp;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct RuntimeSnapshot {
    pub timestamp: Timestamp,
    pub active_workers: usize,
    pub queued_tasks: usize,
}
