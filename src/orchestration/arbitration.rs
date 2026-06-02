use uuid::Uuid;

pub struct ArbitrationDecision {
    pub execution_id:
        Uuid,

    pub winning_node:
        Uuid,
}

pub struct ArbitrationEngine;

impl ArbitrationEngine {
    pub fn decide(
        execution_id:
            Uuid,

        candidates:
            Vec<Uuid>,
    ) -> Option<
        ArbitrationDecision,
    > {
        candidates
            .into_iter()
            .next()
            .map(|winner| {
                ArbitrationDecision {
                    execution_id,

                    winning_node:
                        winner,
                }
            })
    }
}
