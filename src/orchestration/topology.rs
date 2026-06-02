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
pub struct ClusterTopology {
    pub leader:
        Option<Uuid>,

    pub active_nodes:
        Vec<Uuid>,

    pub degraded_nodes:
        Vec<Uuid>,
}
