use chrono::{Duration, Utc};

use serde::{Deserialize, Serialize};

use crate::types::ids::{TaskId, WorkerId};

use crate::types::timestamps::Timestamp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLease {
    pub task_id: TaskId,
    pub worker_id: WorkerId,
    pub acquired_at: Timestamp,
    pub expires_at: Timestamp,
}

impl ExecutionLease {
    pub fn expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn new(task_id: TaskId, worker_id: WorkerId, ttl_seconds: i64) -> Self {
        let now = Utc::now();

        Self {
            task_id,
            worker_id,
            acquired_at: now,
            expires_at: now + Duration::seconds(ttl_seconds),
        }
    }
}
