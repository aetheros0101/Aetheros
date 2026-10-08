use serde::{Deserialize, Serialize};

use crate::types::ids::WorkerId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerEvent {
    WorkerStarted { worker_id: WorkerId },

    WorkerStopped { worker_id: WorkerId },
}
