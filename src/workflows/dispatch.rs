use uuid::Uuid;

use crate::workflows::scheduler::WorkflowPriority;

#[derive(
    Debug,
    Clone,
)]
pub struct DispatchRequest {
    pub workflow_id:
        Uuid,

    pub priority:
        WorkflowPriority,
}

pub struct WorkflowDispatcher;

impl WorkflowDispatcher {
    pub fn dispatch(
        request:
            DispatchRequest,
    ) -> Uuid {
        request.workflow_id
    }
}
