use serde::{
    Deserialize,
    Serialize,
};

use uuid::Uuid;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct RecoveryPlan {
    pub execution_id:
        Uuid,

    pub checkpoint_id:
        Option<Uuid>,

    pub replay_required:
        bool,

    pub failover_required:
        bool,
}
