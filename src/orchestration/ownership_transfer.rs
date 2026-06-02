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
pub struct OwnershipTransfer {
    pub execution_id:
        Uuid,

    pub previous_owner:
        Uuid,

    pub new_owner:
        Uuid,

    pub fence_token:
        u64,
}
