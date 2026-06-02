use uuid::Uuid;

pub struct ReconciliationResult {
    pub execution_id:
        Uuid,

    pub reconciled:
        bool,
}

pub struct ReconciliationEngine;

impl ReconciliationEngine {
    pub fn reconcile(
        execution_id:
            Uuid,
    ) -> ReconciliationResult {
        ReconciliationResult {
            execution_id,

            reconciled: true,
        }
    }
}
