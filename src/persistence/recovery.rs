use crate::persistence::models::PersistedTask;
use crate::task::task::TaskState;

pub struct RecoveryEngine;

impl RecoveryEngine {
    pub fn recoverable_tasks(tasks: Vec<PersistedTask>) -> Vec<PersistedTask> {
        tasks
            .into_iter()
            .filter(|task| !task.is_terminal())
            .filter(|task| task.attempts < task.task.retry_policy.max_attempts)
            .collect()
    }

    pub fn mark_recovered(task: &mut PersistedTask) {
        task.task.state = TaskState::Queued;
    }
}
