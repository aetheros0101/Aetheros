use chrono::Utc;

use crate::task::task::TaskDefinition;

pub fn deadline_expired(task: &TaskDefinition) -> bool {
    match task.deadline {
        Some(deadline) => Utc::now() > deadline,

        None => false,
    }
}
