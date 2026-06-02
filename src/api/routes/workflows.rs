use uuid::Uuid;

use crate::api::models::workflow::WorkflowSubmission;

pub struct WorkflowRoutes;

impl WorkflowRoutes {
    pub fn submit(
        workflow:
            WorkflowSubmission,
    ) -> Uuid {
        workflow.workflow_id
    }
}
